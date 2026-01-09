use super::engine::diff_values;
use super::object::values_equal;
use super::types::{DiffConfig, DiffOp, JsonPath};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Threshold for using HashMap vs linear search for set/multiset operations.
/// Below this size, linear search is faster due to cache locality.
const HASHMAP_THRESHOLD: usize = 20;

/// Compare arrays preserving order using a simple LCS-based approach
pub fn diff_arrays_ordered(
    left: &[Value],
    right: &[Value],
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    // Create wrappers for comparison
    let left_wrapped: Vec<ValueWrapper> = left.iter().map(ValueWrapper).collect();
    let right_wrapped: Vec<ValueWrapper> = right.iter().map(ValueWrapper).collect();

    // Use similar crate for efficient Myers diff
    let changes = similar::capture_diff_slices(
        similar::Algorithm::Myers,
        &left_wrapped,
        &right_wrapped,
    );

    for change in changes {
        match change {
            similar::DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                // Values are equal at top level, but check for deep differences
                for i in 0..len {
                    let child_path = path.append_index(new_index + i);
                    ops.extend(diff_values(
                        &left[old_index + i],
                        &right[new_index + i],
                        &child_path,
                        config,
                    ));
                }
            }
            similar::DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    ops.push(DiffOp::Removed {
                        path: path.append_index(old_index + i),
                        value: left[old_index + i].clone(),
                    });
                }
            }
            similar::DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    ops.push(DiffOp::Added {
                        path: path.append_index(new_index + i),
                        value: right[new_index + i].clone(),
                    });
                }
            }
            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                // Handle replacements as paired modifications where possible
                let min_len = old_len.min(new_len);
                for i in 0..min_len {
                    ops.push(DiffOp::Modified {
                        path: path.append_index(new_index + i),
                        old_value: left[old_index + i].clone(),
                        new_value: right[new_index + i].clone(),
                    });
                }
                // Handle excess deletions
                for i in min_len..old_len {
                    ops.push(DiffOp::Removed {
                        path: path.append_index(old_index + i),
                        value: left[old_index + i].clone(),
                    });
                }
                // Handle excess insertions
                for i in min_len..new_len {
                    ops.push(DiffOp::Added {
                        path: path.append_index(new_index + i),
                        value: right[new_index + i].clone(),
                    });
                }
            }
        }
    }

    ops
}

/// Compare arrays as sets (order ignored, duplicates ignored)
pub fn diff_arrays_as_set(
    left: &[Value],
    right: &[Value],
    path: &JsonPath,
    _config: &DiffConfig,
) -> Vec<DiffOp> {
    // For small arrays, use linear search (cache-friendly)
    if left.len() < HASHMAP_THRESHOLD && right.len() < HASHMAP_THRESHOLD {
        return diff_arrays_as_set_linear(left, right, path);
    }

    let mut ops = Vec::new();

    // Build hash maps for O(1) average lookups
    // Map: hash -> list of values (to handle collisions)
    let left_map = build_value_set(left);
    let right_map = build_value_set(right);

    // Find elements only in left (removed) - iterate in input order for determinism
    for val in left {
        let hash = compute_value_hash(val);
        // Only process first occurrence of each unique value
        if is_first_occurrence_in_map(&left_map, hash, val) && !set_contains(&right_map, hash, val) {
            ops.push(DiffOp::Removed {
                path: path.append_set_marker(),
                value: val.clone(),
            });
        }
    }

    // Find elements only in right (added) - iterate in input order for determinism
    for val in right {
        let hash = compute_value_hash(val);
        // Only process first occurrence of each unique value
        if is_first_occurrence_in_map(&right_map, hash, val) && !set_contains(&left_map, hash, val) {
            ops.push(DiffOp::Added {
                path: path.append_set_marker(),
                value: val.clone(),
            });
        }
    }

    ops
}

/// Linear set comparison for small arrays
fn diff_arrays_as_set_linear(left: &[Value], right: &[Value], path: &JsonPath) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    // Build sets using linear search
    let mut left_set: Vec<&Value> = Vec::new();
    for val in left {
        if !left_set.iter().any(|v| values_equal(v, val)) {
            left_set.push(val);
        }
    }

    let mut right_set: Vec<&Value> = Vec::new();
    for val in right {
        if !right_set.iter().any(|v| values_equal(v, val)) {
            right_set.push(val);
        }
    }

    // Find removed (in left order)
    for val in &left_set {
        if !right_set.iter().any(|v| values_equal(v, val)) {
            ops.push(DiffOp::Removed {
                path: path.append_set_marker(),
                value: (*val).clone(),
            });
        }
    }

    // Find added (in right order)
    for val in &right_set {
        if !left_set.iter().any(|v| values_equal(v, val)) {
            ops.push(DiffOp::Added {
                path: path.append_set_marker(),
                value: (*val).clone(),
            });
        }
    }

    ops
}

/// Compare arrays as multisets (order ignored, duplicates counted)
pub fn diff_arrays_as_multiset(
    left: &[Value],
    right: &[Value],
    path: &JsonPath,
    _config: &DiffConfig,
) -> Vec<DiffOp> {
    // For small arrays, use linear search (cache-friendly)
    if left.len() < HASHMAP_THRESHOLD && right.len() < HASHMAP_THRESHOLD {
        return diff_arrays_as_multiset_linear(left, right, path);
    }

    let mut ops = Vec::new();

    // Count occurrences using hash-based approach
    let left_counts = count_values_hashmap(left);
    let right_counts = count_values_hashmap(right);

    // Process left values in input order for determinism
    // Use HashSet for O(1) lookup of processed hashes
    let mut processed: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for val in left {
        let hash = compute_value_hash(val);
        // Only process first occurrence of each unique value
        if !processed.contains(&hash) && is_first_occurrence_in_count_map(&left_counts, hash, val) {
            let left_count = get_count(&left_counts, hash, val);
            let right_count = get_count(&right_counts, hash, val);

            if left_count > right_count {
                for _ in 0..(left_count - right_count) {
                    ops.push(DiffOp::Removed {
                        path: path.append_multiset_marker(),
                        value: val.clone(),
                    });
                }
            }
            processed.insert(hash);
        }
    }

    // Process right values for additions
    processed.clear();
    for val in right {
        let hash = compute_value_hash(val);
        if !processed.contains(&hash) && is_first_occurrence_in_count_map(&right_counts, hash, val)
        {
            let left_count = get_count(&left_counts, hash, val);
            let right_count = get_count(&right_counts, hash, val);

            if right_count > left_count {
                for _ in 0..(right_count - left_count) {
                    ops.push(DiffOp::Added {
                        path: path.append_multiset_marker(),
                        value: val.clone(),
                    });
                }
            }
            processed.insert(hash);
        }
    }

    ops
}

/// Linear multiset comparison for small arrays
fn diff_arrays_as_multiset_linear(
    left: &[Value],
    right: &[Value],
    path: &JsonPath,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let left_counts = count_values_linear(left);
    let right_counts = count_values_linear(right);

    // Collect all unique values
    let mut all_values: Vec<&Value> = Vec::new();
    for (val, _) in &left_counts {
        if !all_values.iter().any(|v| values_equal(v, val)) {
            all_values.push(val);
        }
    }
    for (val, _) in &right_counts {
        if !all_values.iter().any(|v| values_equal(v, val)) {
            all_values.push(val);
        }
    }

    // Compare counts
    for val in all_values {
        let left_count = left_counts
            .iter()
            .find(|(v, _)| values_equal(v, val))
            .map(|(_, c)| *c)
            .unwrap_or(0);
        let right_count = right_counts
            .iter()
            .find(|(v, _)| values_equal(v, val))
            .map(|(_, c)| *c)
            .unwrap_or(0);

        if left_count > right_count {
            for _ in 0..(left_count - right_count) {
                ops.push(DiffOp::Removed {
                    path: path.append_multiset_marker(),
                    value: val.clone(),
                });
            }
        } else if right_count > left_count {
            for _ in 0..(right_count - left_count) {
                ops.push(DiffOp::Added {
                    path: path.append_multiset_marker(),
                    value: val.clone(),
                });
            }
        }
    }

    ops
}

/// Count occurrences using linear search (for small arrays)
fn count_values_linear(values: &[Value]) -> Vec<(&Value, usize)> {
    let mut counts: Vec<(&Value, usize)> = Vec::new();
    for val in values {
        if let Some((_, count)) = counts.iter_mut().find(|(v, _)| values_equal(v, val)) {
            *count += 1;
        } else {
            counts.push((val, 1));
        }
    }
    counts
}

// ============================================================================
// Hash-based helper functions for O(1) lookups
// ============================================================================

/// Compute hash for a JSON value
fn compute_value_hash(value: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_value(value, &mut hasher);
    hasher.finish()
}

/// Build a hash map for set operations (deduplicates values)
/// Returns: hash -> Vec<&Value> (multiple values possible due to collisions)
fn build_value_set(values: &[Value]) -> HashMap<u64, Vec<&Value>> {
    let mut map: HashMap<u64, Vec<&Value>> = HashMap::new();

    for val in values {
        let hash = compute_value_hash(val);
        let entry = map.entry(hash).or_default();
        // Deduplicate within same hash bucket
        if !entry.iter().any(|v| values_equal(v, val)) {
            entry.push(val);
        }
    }

    map
}

/// Check if a value exists in the set (handles hash collisions)
fn set_contains(map: &HashMap<u64, Vec<&Value>>, hash: u64, value: &Value) -> bool {
    match map.get(&hash) {
        Some(vals) => vals.iter().any(|v| values_equal(v, value)),
        None => false,
    }
}

/// Check if this is the first occurrence of the value in the map
fn is_first_occurrence_in_map(map: &HashMap<u64, Vec<&Value>>, hash: u64, value: &Value) -> bool {
    match map.get(&hash) {
        Some(vals) => {
            // Find the first value that equals this one
            vals.first().map(|v| values_equal(v, value)).unwrap_or(false)
        }
        None => false,
    }
}

/// Count occurrences using hash-based approach
/// Returns: hash -> Vec<(&Value, count)>
fn count_values_hashmap(values: &[Value]) -> HashMap<u64, Vec<(&Value, usize)>> {
    let mut map: HashMap<u64, Vec<(&Value, usize)>> = HashMap::new();

    for val in values {
        let hash = compute_value_hash(val);
        let entry = map.entry(hash).or_default();

        // Find existing count for this exact value (handling collisions)
        if let Some((_, count)) = entry.iter_mut().find(|(v, _)| values_equal(v, val)) {
            *count += 1;
        } else {
            entry.push((val, 1));
        }
    }

    map
}

/// Get count of a value from the count map
fn get_count(map: &HashMap<u64, Vec<(&Value, usize)>>, hash: u64, value: &Value) -> usize {
    match map.get(&hash) {
        Some(entries) => entries
            .iter()
            .find(|(v, _)| values_equal(v, value))
            .map(|(_, c)| *c)
            .unwrap_or(0),
        None => 0,
    }
}

/// Check if this is the first occurrence in the count map
fn is_first_occurrence_in_count_map(
    map: &HashMap<u64, Vec<(&Value, usize)>>,
    hash: u64,
    value: &Value,
) -> bool {
    match map.get(&hash) {
        Some(entries) => entries
            .first()
            .map(|(v, _)| values_equal(v, value))
            .unwrap_or(false),
        None => false,
    }
}

/// Wrapper for Value to implement Hash and Eq for similar crate
#[derive(Clone)]
pub struct ValueWrapper<'a>(pub &'a Value);

impl<'a> PartialEq for ValueWrapper<'a> {
    fn eq(&self, other: &Self) -> bool {
        values_equal(self.0, other.0)
    }
}

impl<'a> Eq for ValueWrapper<'a> {}

impl<'a> Hash for ValueWrapper<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_value(self.0, state);
    }
}

impl<'a> PartialOrd for ValueWrapper<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for ValueWrapper<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Use the hash as a proxy for ordering
        use std::collections::hash_map::DefaultHasher;
        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        self.hash(&mut hasher1);
        other.hash(&mut hasher2);
        hasher1.finish().cmp(&hasher2.finish())
    }
}

fn hash_value<H: Hasher>(value: &Value, state: &mut H) {
    if value.is_null() {
        0u8.hash(state);
    } else if value.is_boolean() {
        1u8.hash(state);
        value.as_bool().unwrap_or(false).hash(state);
    } else if value.is_number() {
        2u8.hash(state);
        format!("{}", value).hash(state);
    } else if value.is_str() {
        3u8.hash(state);
        value.as_str().unwrap_or("").hash(state);
    } else if value.is_array() {
        4u8.hash(state);
        if let Some(arr) = value.as_array() {
            arr.len().hash(state);
            for v in arr {
                hash_value(v, state);
            }
        }
    } else if value.is_object() {
        5u8.hash(state);
        if let Some(obj) = value.as_object() {
            // Sort keys for consistent hash
            let mut keys: Vec<&str> = obj.iter().map(|(k, _)| k).collect();
            keys.sort();
            for k in keys {
                k.hash(state);
                if let Some(v) = obj.get(&k.to_string()) {
                    hash_value(v, state);
                }
            }
        }
    }
}
