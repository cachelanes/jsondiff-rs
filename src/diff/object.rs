use super::engine::diff_values;
use super::types::{DiffConfig, DiffOp, JsonPath};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Object, Value};
use std::collections::HashSet;

/// Compare two JSON objects
/// By default, objects are compared unordered (keys matched by name)
pub fn diff_objects(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k).collect();
    let right_keys: HashSet<&str> = right.iter().map(|(k, _)| k).collect();

    // Keys only in left (removed)
    for key in left_keys.difference(&right_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Removed {
            path: child_path,
            value: left.get(key).unwrap().clone(),
        });
    }

    // Keys only in right (added)
    for key in right_keys.difference(&left_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Added {
            path: child_path,
            value: right.get(key).unwrap().clone(),
        });
    }

    // Keys in both (recurse or check order)
    if config.ordered_objects {
        // Compare in order - if keys are in different positions, treat as modified
        let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
        let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

        // Only compare common keys
        let common_keys: HashSet<&str> = left_keys.intersection(&right_keys).copied().collect();

        for key in &common_keys {
            let left_pos = left_order.iter().position(|k| k == key);
            let right_pos = right_order.iter().position(|k| k == key);

            if left_pos != right_pos {
                // Key position changed - report as modified
                let child_path = path.append_key(key);
                let left_val = left.get(key).unwrap();
                let right_val = right.get(key).unwrap();

                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                // Same position, recurse
                let child_path = path.append_key(key);
                let left_val = left.get(key).unwrap();
                let right_val = right.get(key).unwrap();
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        }
    } else {
        // Unordered comparison - just compare values for common keys
        for key in left_keys.intersection(&right_keys) {
            let child_path = path.append_key(key);
            let left_val = left.get(key).unwrap();
            let right_val = right.get(key).unwrap();
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        }
    }

    ops
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
