use super::engine::diff_values;
use super::object::values_equal;
use super::types::{DiffConfig, DiffOp, JsonPath};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use std::hash::{Hash, Hasher};

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
    let mut ops = Vec::new();

    // Build sets using our custom equality
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

    // Find elements only in left (removed)
    for val in &left_set {
        if !right_set.iter().any(|v| values_equal(v, val)) {
            ops.push(DiffOp::Removed {
                path: path.append_set_marker(),
                value: (*val).clone(),
            });
        }
    }

    // Find elements only in right (added)
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
    let mut ops = Vec::new();

    // Count occurrences using a list-based approach for proper equality
    let left_counts = count_values(left);
    let right_counts = count_values(right);

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

/// Count occurrences of each value in an array
fn count_values(values: &[Value]) -> Vec<(&Value, usize)> {
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
