use super::array::{diff_arrays_as_multiset, diff_arrays_as_set, diff_arrays_ordered, diff_arrays_with_set_key};
use super::object::diff_objects;
use super::types::{ArrayCompareMode, DiffConfig, DiffOp, DiffResult, DiffStats, JsonPath};
use crate::error::JsonDiffError;
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};

pub struct DiffEngine {
    config: DiffConfig,
}

impl DiffEngine {
    pub fn new(config: DiffConfig) -> Self {
        Self { config }
    }

    pub fn diff(&self, left: &Value, right: &Value) -> Result<DiffResult, JsonDiffError> {
        let path = JsonPath::root();
        let operations = diff_values(left, right, &path, &self.config)?;
        let stats = Self::compute_stats(&operations);
        Ok(DiffResult { operations, stats })
    }

    fn compute_stats(operations: &[DiffOp]) -> DiffStats {
        let mut stats = DiffStats::default();
        for op in operations {
            match op {
                DiffOp::Added { .. } => stats.added += 1,
                DiffOp::Removed { .. } => stats.removed += 1,
                DiffOp::Modified { .. } => stats.modified += 1,
            }
        }
        stats
    }
}

/// Compare two JSON values and return the differences
pub fn diff_values(left: &Value, right: &Value, path: &JsonPath, config: &DiffConfig) -> Result<Vec<DiffOp>, JsonDiffError> {
    // If values are equal, no diff
    if left == right {
        return Ok(vec![]);
    }

    // Check type compatibility
    let left_type = get_value_type(left);
    let right_type = get_value_type(right);

    // Type mismatch - complete replacement
    if left_type != right_type {
        return Ok(vec![DiffOp::Modified {
            path: path.clone(),
            old_value: left.clone(),
            new_value: right.clone(),
        }]);
    }

    // Same type, compare based on type
    if left.is_object() && right.is_object() {
        let l = left.as_object().unwrap();
        let r = right.as_object().unwrap();
        return diff_objects(l, r, path, config);
    }

    if left.is_array() && right.is_array() {
        let l = left.as_array().unwrap();
        let r = right.as_array().unwrap();
        let left_slice: Vec<Value> = l.iter().cloned().collect();
        let right_slice: Vec<Value> = r.iter().cloned().collect();

        // Check if this array path matches a set-key config
        if let Some(ref set_key_config) = config.set_keys {
            let path_str = path.to_set_key_path();
            if let Some(key_fields) = set_key_config.paths.get(&path_str) {
                return diff_arrays_with_set_key(
                    &left_slice,
                    &right_slice,
                    path,
                    config,
                    key_fields,
                    set_key_config,
                );
            }
        }

        return match config.array_mode {
            ArrayCompareMode::Ordered => {
                diff_arrays_ordered(&left_slice, &right_slice, path, config)
            }
            ArrayCompareMode::Set => Ok(diff_arrays_as_set(&left_slice, &right_slice, path, config)),
            ArrayCompareMode::MultiSet => {
                Ok(diff_arrays_as_multiset(&left_slice, &right_slice, path, config))
            }
        };
    }

    // Primitives: null, bool, number, string
    Ok(vec![DiffOp::Modified {
        path: path.clone(),
        old_value: left.clone(),
        new_value: right.clone(),
    }])
}

/// Get the type of a JSON value as a string for comparison
fn get_value_type(value: &Value) -> &'static str {
    if value.is_null() {
        "null"
    } else if value.is_boolean() {
        "boolean"
    } else if value.is_number() {
        "number"
    } else if value.is_str() {
        "string"
    } else if value.is_array() {
        "array"
    } else if value.is_object() {
        "object"
    } else {
        "unknown"
    }
}
