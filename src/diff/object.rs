use super::engine::diff_values;
use super::types::{DiffConfig, DiffOp, JsonPath};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Object, Value};
use std::collections::HashSet;

/// Threshold for using HashSet vs linear search for key membership checks.
/// Below this size, linear search is faster due to cache locality and avoiding
/// HashSet construction overhead. Value chosen based on typical stdlib thresholds.
const HASHSET_THRESHOLD: usize = 20;

/// Compare two JSON objects
/// By default, objects are compared unordered (keys matched by name)
/// Output order follows the input JSON structure (left file for removed/common keys,
/// right file for added keys) for deterministic, intuitive results.
pub fn diff_objects(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    // For small objects, linear search is faster than HashSet construction + lookup
    let use_hashset = left.len() >= HASHSET_THRESHOLD || right.len() >= HASHSET_THRESHOLD;

    if use_hashset {
        // Build key sets once for O(1) membership checks (avoids repeated String allocations)
        let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k).collect();
        let right_keys: HashSet<&str> = right.iter().map(|(k, _)| k).collect();

        if config.ordered_objects {
            diff_objects_ordered_hashset(left, right, path, config, &mut ops, &left_keys, &right_keys);
        } else {
            diff_objects_unordered_hashset(left, right, path, config, &mut ops, &left_keys, &right_keys);
        }
    } else {
        // Linear search for small objects
        if config.ordered_objects {
            diff_objects_ordered_linear(left, right, path, config, &mut ops);
        } else {
            diff_objects_unordered_linear(left, right, path, config, &mut ops);
        }
    }

    ops
}

/// Unordered comparison for small objects using linear search
fn diff_objects_unordered_linear(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
) {
    // Process keys in left object order: removed keys and common keys
    for (key, left_val) in left.iter() {
        let child_path = path.append_key(key);
        if let Some(right_val) = right.get(&key.to_string()) {
            // Common key - recurse to compare values
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        } else {
            // Key only in left - removed
            ops.push(DiffOp::Removed {
                path: child_path,
                value: left_val.clone(),
            });
        }
    }

    // Process keys only in right object (added) - in right's order
    for (key, right_val) in right.iter() {
        if left.get(&key.to_string()).is_none() {
            let child_path = path.append_key(key);
            ops.push(DiffOp::Added {
                path: child_path,
                value: right_val.clone(),
            });
        }
    }
}

/// Unordered comparison for larger objects using HashSet for O(1) lookups
fn diff_objects_unordered_hashset(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
    left_keys: &HashSet<&str>,
    right_keys: &HashSet<&str>,
) {
    // Process keys in left object order: removed keys and common keys
    for (key, left_val) in left.iter() {
        let child_path = path.append_key(key);
        if right_keys.contains(key) {
            // Common key - recurse to compare values
            let right_val = right.get(&key.to_string()).unwrap();
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        } else {
            // Key only in left - removed
            ops.push(DiffOp::Removed {
                path: child_path,
                value: left_val.clone(),
            });
        }
    }

    // Process keys only in right object (added) - in right's order
    for (key, right_val) in right.iter() {
        if !left_keys.contains(key) {
            let child_path = path.append_key(key);
            ops.push(DiffOp::Added {
                path: child_path,
                value: right_val.clone(),
            });
        }
    }
}

/// Ordered comparison for small objects using linear search
fn diff_objects_ordered_linear(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
) {
    let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
    let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

    // Process keys in left object order
    for (key, left_val) in left.iter() {
        let child_path = path.append_key(key);
        if let Some(right_val) = right.get(&key.to_string()) {
            // Common key - check position
            let left_pos = left_order.iter().position(|k| *k == key);
            let right_pos = right_order.iter().position(|k| *k == key);

            if left_pos != right_pos {
                // Key position changed - report as modified if values also differ
                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                // Same position - recurse
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        } else {
            // Key only in left - removed
            ops.push(DiffOp::Removed {
                path: child_path,
                value: left_val.clone(),
            });
        }
    }

    // Process keys only in right object (added) - in right's order
    for (key, right_val) in right.iter() {
        if left.get(&key.to_string()).is_none() {
            let child_path = path.append_key(key);
            ops.push(DiffOp::Added {
                path: child_path,
                value: right_val.clone(),
            });
        }
    }
}

/// Ordered comparison for larger objects using HashSet for O(1) lookups
fn diff_objects_ordered_hashset(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
    ops: &mut Vec<DiffOp>,
    left_keys: &HashSet<&str>,
    right_keys: &HashSet<&str>,
) {
    let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
    let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

    // Process keys in left object order
    for (key, left_val) in left.iter() {
        let child_path = path.append_key(key);
        if right_keys.contains(key) {
            // Common key - check position
            let right_val = right.get(&key.to_string()).unwrap();
            let left_pos = left_order.iter().position(|k| *k == key);
            let right_pos = right_order.iter().position(|k| *k == key);

            if left_pos != right_pos {
                // Key position changed - report as modified if values also differ
                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                // Same position - recurse
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        } else {
            // Key only in left - removed
            ops.push(DiffOp::Removed {
                path: child_path,
                value: left_val.clone(),
            });
        }
    }

    // Process keys only in right object (added) - in right's order
    for (key, right_val) in right.iter() {
        if !left_keys.contains(key) {
            let child_path = path.append_key(key);
            ops.push(DiffOp::Added {
                path: child_path,
                value: right_val.clone(),
            });
        }
    }
}

/// Check if two values are equal (deep comparison)
pub fn values_equal(left: &Value, right: &Value) -> bool {
    if left.is_null() && right.is_null() {
        return true;
    }
    if left.is_boolean() && right.is_boolean() {
        return left.as_bool() == right.as_bool();
    }
    if left.is_number() && right.is_number() {
        // Compare numbers by their string representation to handle precision
        return format!("{}", left) == format!("{}", right);
    }
    if left.is_str() && right.is_str() {
        return left.as_str() == right.as_str();
    }
    if left.is_array() && right.is_array() {
        let l = left.as_array().unwrap();
        let r = right.as_array().unwrap();
        if l.len() != r.len() {
            return false;
        }
        return l.iter().zip(r.iter()).all(|(a, b)| values_equal(a, b));
    }
    if left.is_object() && right.is_object() {
        let l = left.as_object().unwrap();
        let r = right.as_object().unwrap();
        if l.len() != r.len() {
            return false;
        }
        return l.iter().all(|(k, v)| {
            r.get(&k.to_string())
                .map(|rv| values_equal(v, rv))
                .unwrap_or(false)
        });
    }
    false
}
