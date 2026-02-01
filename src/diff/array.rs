use super::engine::diff_values;
use super::object::values_equal;
use super::types::{DiffConfig, DiffOp, JsonPath, SetKeyConfig};
use crate::error::JsonDiffError;
use imara_diff::{Algorithm, Diff, InternedInput};
use sonic_rs::{JsonContainerTrait, JsonValueTrait, Value};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Threshold for using HashMap vs linear search for set/multiset operations.
/// Below this size, linear search is faster due to cache locality.
const HASHMAP_THRESHOLD: usize = 20;

/// Threshold for using HashSet in set-key duplicate detection.
/// Aligned with HASHMAP_THRESHOLD used by set/multiset modes.
const SET_KEY_HASHMAP_THRESHOLD: usize = HASHMAP_THRESHOLD;

/// Compare arrays preserving order using a simple LCS-based approach
pub fn diff_arrays_ordered(
    left: &[Value],
    right: &[Value],
    path: &JsonPath,
    config: &DiffConfig,
) -> Result<Vec<DiffOp>, JsonDiffError> {
    // =========================================================================
    // Phase 1: Early termination paths
    // =========================================================================

    // Fast path: both arrays empty
    if left.is_empty() && right.is_empty() {
        return Ok(vec![]);
    }

    // Fast path: left array empty - all elements are additions
    if left.is_empty() {
        return Ok(right
            .iter()
            .enumerate()
            .map(|(i, val)| DiffOp::Added {
                path: path.append_index(i),
                value: val.clone(),
            })
            .collect());
    }

    // Fast path: right array empty - all elements are removals
    if right.is_empty() {
        return Ok(left
            .iter()
            .enumerate()
            .map(|(i, val)| DiffOp::Removed {
                path: path.append_index(i),
                value: val.clone(),
            })
            .collect());
    }

    // =========================================================================
    // Phase 2: Prefix/suffix matching - reduce problem size
    // =========================================================================

    // Find common prefix (identical leading elements)
    let prefix_len = find_common_prefix(left, right);

    // Find common suffix (identical trailing elements, not overlapping prefix)
    let suffix_len = find_common_suffix(left, right, prefix_len);

    // If prefix + suffix covers everything, arrays are identical
    if prefix_len + suffix_len >= left.len() && prefix_len + suffix_len >= right.len() {
        // All elements match - just check for deep differences in prefix region
        let mut ops = Vec::new();
        for i in 0..left.len() {
            let child_path = path.append_index(i);
            ops.extend(diff_values(&left[i], &right[i], &child_path, config)?);
        }
        return Ok(ops);
    }

    // Get the middle portion that needs diff
    let left_middle_end = left.len() - suffix_len;
    let right_middle_end = right.len() - suffix_len;
    let left_middle = &left[prefix_len..left_middle_end];
    let right_middle = &right[prefix_len..right_middle_end];

    let mut ops = Vec::new();

    // Process prefix: check for deep differences
    for i in 0..prefix_len {
        let child_path = path.append_index(i);
        ops.extend(diff_values(&left[i], &right[i], &child_path, config)?);
    }

    // Process the middle portion with diff algorithm
    // Phase 3: Use pre-computed hashes for fast comparison
    // Phase 5: Use imara-diff histogram algorithm (faster for spread diffs)
    if !left_middle.is_empty() || !right_middle.is_empty() {
        // Pre-compute hashes for all elements
        let left_hashes: Vec<u64> = left_middle.iter().map(compute_value_hash).collect();
        let right_hashes: Vec<u64> = right_middle.iter().map(compute_value_hash).collect();

        // Use imara-diff with histogram algorithm on hashes
        let mut input: InternedInput<u64> = InternedInput::default();
        input.update_before(left_hashes.iter().copied());
        input.update_after(right_hashes.iter().copied());

        // Collect hunks from imara-diff
        let diff = Diff::compute(Algorithm::Histogram, &input);
        let hunks: Vec<_> = diff.hunks().map(|h| (h.before, h.after)).collect();

        // Process hunks to generate DiffOps
        let mut left_pos: usize = 0;
        let mut right_pos: usize = 0;

        for (before_range, after_range) in hunks {
            let before_start = before_range.start as usize;
            let before_end = before_range.end as usize;
            let after_start = after_range.start as usize;
            let after_end = after_range.end as usize;

            // Process equal elements before this hunk
            while left_pos < before_start && right_pos < after_start {
                let actual_left_idx = prefix_len + left_pos;
                let actual_right_idx = prefix_len + right_pos;
                let child_path = path.append_index(actual_right_idx);
                ops.extend(diff_values(
                    &left[actual_left_idx],
                    &right[actual_right_idx],
                    &child_path,
                    config,
                )?);
                left_pos += 1;
                right_pos += 1;
            }

            // Process the hunk itself
            let old_len = before_end - before_start;
            let new_len = after_end - after_start;

            if old_len == 0 {
                // Pure insertion
                for i in 0..new_len {
                    let actual_idx = prefix_len + after_start + i;
                    ops.push(DiffOp::Added {
                        path: path.append_index(actual_idx),
                        value: right[actual_idx].clone(),
                    });
                }
            } else if new_len == 0 {
                // Pure deletion
                for i in 0..old_len {
                    let actual_idx = prefix_len + before_start + i;
                    ops.push(DiffOp::Removed {
                        path: path.append_index(actual_idx),
                        value: left[actual_idx].clone(),
                    });
                }
            } else {
                // Replacement: pair up as modifications, then handle excess
                let min_len = old_len.min(new_len);
                for i in 0..min_len {
                    let actual_left_idx = prefix_len + before_start + i;
                    let actual_right_idx = prefix_len + after_start + i;
                    ops.push(DiffOp::Modified {
                        path: path.append_index(actual_right_idx),
                        old_value: left[actual_left_idx].clone(),
                        new_value: right[actual_right_idx].clone(),
                    });
                }
                // Excess deletions
                for i in min_len..old_len {
                    let actual_idx = prefix_len + before_start + i;
                    ops.push(DiffOp::Removed {
                        path: path.append_index(actual_idx),
                        value: left[actual_idx].clone(),
                    });
                }
                // Excess insertions
                for i in min_len..new_len {
                    let actual_idx = prefix_len + after_start + i;
                    ops.push(DiffOp::Added {
                        path: path.append_index(actual_idx),
                        value: right[actual_idx].clone(),
                    });
                }
            }

            left_pos = before_end;
            right_pos = after_end;
        }

        // Process remaining equal elements after all hunks
        while left_pos < left_middle.len() && right_pos < right_middle.len() {
            let actual_left_idx = prefix_len + left_pos;
            let actual_right_idx = prefix_len + right_pos;
            let child_path = path.append_index(actual_right_idx);
            ops.extend(diff_values(
                &left[actual_left_idx],
                &right[actual_right_idx],
                &child_path,
                config,
            )?);
            left_pos += 1;
            right_pos += 1;
        }
    }

    // Process suffix: check for deep differences
    for i in 0..suffix_len {
        let left_idx = left.len() - suffix_len + i;
        let right_idx = right.len() - suffix_len + i;
        let child_path = path.append_index(right_idx);
        ops.extend(diff_values(&left[left_idx], &right[right_idx], &child_path, config)?);
    }

    Ok(ops)
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

// ============================================================================
// Set-key based array comparison
// ============================================================================

/// Compare arrays by matching objects on key fields instead of position.
/// Elements with matching keys are recursively compared; unmatched elements
/// are reported as added/removed. Elements missing key fields fall back
/// to set comparison (lenient) or error (strict).
pub fn diff_arrays_with_set_key(
    left: &[Value],
    right: &[Value],
    path: &JsonPath,
    config: &DiffConfig,
    key_fields: &[String],
    set_key_config: &SetKeyConfig,
) -> Result<Vec<DiffOp>, JsonDiffError> {
    let mut ops = Vec::new();

    // Partition elements into keyed (have all key fields) and unkeyed.
    let mut left_keyed: Vec<(Vec<(String, String)>, &Value)> = Vec::new();
    let mut left_unkeyed: Vec<&Value> = Vec::new();

    for (i, val) in left.iter().enumerate() {
        match extract_set_key(val, key_fields) {
            Some(key) => left_keyed.push((key, val)),
            None => {
                if !set_key_config.allow_missing {
                    let missing_field = find_first_missing_field(val, key_fields);
                    return Err(JsonDiffError::SetKeyMissing {
                        path: format!("{}[{}]", path, i),
                        field: missing_field,
                    });
                }
                left_unkeyed.push(val);
            }
        }
    }

    let mut right_keyed: Vec<(Vec<(String, String)>, &Value)> = Vec::new();
    let mut right_unkeyed: Vec<&Value> = Vec::new();

    for (i, val) in right.iter().enumerate() {
        match extract_set_key(val, key_fields) {
            Some(key) => right_keyed.push((key, val)),
            None => {
                if !set_key_config.allow_missing {
                    let missing_field = find_first_missing_field(val, key_fields);
                    return Err(JsonDiffError::SetKeyMissing {
                        path: format!("{}[{}]", path, i),
                        field: missing_field,
                    });
                }
                right_unkeyed.push(val);
            }
        }
    }

    // Build key→value maps (first-match-wins for duplicates).
    // Use linear scan for small arrays (cache-friendly), HashSet for large ones.
    let use_hashset = left_keyed.len() >= SET_KEY_HASHMAP_THRESHOLD
        || right_keyed.len() >= SET_KEY_HASHMAP_THRESHOLD;

    let mut left_map: Vec<(String, Vec<(String, String)>, &Value)> = Vec::new();
    let mut right_map: Vec<(String, Vec<(String, String)>, &Value)> = Vec::new();

    if use_hashset {
        let mut left_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (key, val) in &left_keyed {
            let key_str = set_key_to_string(key);
            if !left_seen.insert(key_str.clone()) {
                if !set_key_config.allow_duplicates {
                    return Err(JsonDiffError::SetKeyDuplicate {
                        key: key_str.clone(),
                        path1: format!("{}", path),
                        path2: format!("{}", path),
                    });
                }
                continue;
            }
            left_map.push((key_str, key.clone(), val));
        }

        let mut right_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (key, val) in &right_keyed {
            let key_str = set_key_to_string(key);
            if !right_seen.insert(key_str.clone()) {
                if !set_key_config.allow_duplicates {
                    return Err(JsonDiffError::SetKeyDuplicate {
                        key: key_str.clone(),
                        path1: format!("{}", path),
                        path2: format!("{}", path),
                    });
                }
                continue;
            }
            right_map.push((key_str, key.clone(), val));
        }
    } else {
        for (key, val) in &left_keyed {
            let key_str = set_key_to_string(key);
            if left_map.iter().any(|(k, _, _)| *k == key_str) {
                if !set_key_config.allow_duplicates {
                    return Err(JsonDiffError::SetKeyDuplicate {
                        key: key_str.clone(),
                        path1: format!("{}", path),
                        path2: format!("{}", path),
                    });
                }
                continue;
            }
            left_map.push((key_str, key.clone(), val));
        }

        for (key, val) in &right_keyed {
            let key_str = set_key_to_string(key);
            if right_map.iter().any(|(k, _, _)| *k == key_str) {
                if !set_key_config.allow_duplicates {
                    return Err(JsonDiffError::SetKeyDuplicate {
                        key: key_str.clone(),
                        path1: format!("{}", path),
                        path2: format!("{}", path),
                    });
                }
                continue;
            }
            right_map.push((key_str, key.clone(), val));
        }
    }

    // Find matched pairs and compare recursively (left order for determinism)
    for (key_str, key_parts, left_val) in &left_map {
        if let Some((_, _, right_val)) = right_map.iter().find(|(k, _, _)| k == key_str) {
            let child_path = path.append_key_match(key_parts.clone());
            ops.extend(diff_values(left_val, right_val, &child_path, config)?);
        } else {
            // Only in left → removed
            ops.push(DiffOp::Removed {
                path: path.append_key_match(key_parts.clone()),
                value: (*left_val).clone(),
            });
        }
    }

    // Find elements only in right → added (right order for determinism)
    for (key_str, key_parts, right_val) in &right_map {
        if !left_map.iter().any(|(k, _, _)| k == key_str) {
            ops.push(DiffOp::Added {
                path: path.append_key_match(key_parts.clone()),
                value: (*right_val).clone(),
            });
        }
    }

    // Handle unkeyed elements via set comparison with [{}] marker
    if !left_unkeyed.is_empty() || !right_unkeyed.is_empty() {
        let set_ops = diff_unkeyed_as_set(&left_unkeyed, &right_unkeyed, path);
        ops.extend(set_ops);
    }

    Ok(ops)
}

/// Extract key field values from a JSON value. Returns None if the value
/// is not an object or is missing any of the required key fields.
fn extract_set_key(value: &Value, key_fields: &[String]) -> Option<Vec<(String, String)>> {
    if !value.is_object() {
        return None;
    }
    let obj = value.as_object().unwrap();
    let mut key_parts = Vec::with_capacity(key_fields.len());
    for field in key_fields {
        match obj.get(&field.to_string()) {
            Some(v) => {
                let key_str = value_to_key_string(v);
                key_parts.push((field.clone(), key_str));
            }
            None => return None,
        }
    }
    Some(key_parts)
}

/// Convert a JSON value to its string representation for use as a key.
fn value_to_key_string(value: &Value) -> String {
    if value.is_null() {
        "null".to_string()
    } else if value.is_boolean() {
        value.as_bool().unwrap().to_string()
    } else if value.is_number() {
        format!("{}", value)
    } else if value.is_str() {
        value.as_str().unwrap().to_string()
    } else {
        format!("{}", value)
    }
}

/// Convert a set key (list of field/value pairs) to a single lookup string.
fn set_key_to_string(key: &[(String, String)]) -> String {
    key.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(",")
}

/// Find the first missing key field in a value (for error messages).
fn find_first_missing_field(value: &Value, key_fields: &[String]) -> String {
    if !value.is_object() {
        return key_fields.first().cloned().unwrap_or_default();
    }
    let obj = value.as_object().unwrap();
    for field in key_fields {
        if obj.get(&field.to_string()).is_none() {
            return field.clone();
        }
    }
    key_fields.first().cloned().unwrap_or_default()
}

/// Compare unkeyed elements as a set (for fallback in lenient mode).
/// Uses the [{}] set marker in paths.
fn diff_unkeyed_as_set(left: &[&Value], right: &[&Value], path: &JsonPath) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    // Find elements only in left (removed)
    for val in left {
        if !right.iter().any(|r| values_equal(val, r)) {
            ops.push(DiffOp::Removed {
                path: path.append_set_marker(),
                value: (*val).clone(),
            });
        }
    }

    // Find elements only in right (added)
    for val in right {
        if !left.iter().any(|l| values_equal(val, l)) {
            ops.push(DiffOp::Added {
                path: path.append_set_marker(),
                value: (*val).clone(),
            });
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

// Note: ValueWrapper and PrehashedWrapper removed as ordered mode now uses
// hash-based comparison with imara-diff histogram algorithm (Phase 5)

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


// ============================================================================
// Helper functions for prefix/suffix matching (Phase 2)
// (Phase 1 hash-based early termination is now handled by prefix/suffix)
// ============================================================================

/// Find the length of the common prefix (identical leading elements)
fn find_common_prefix(left: &[Value], right: &[Value]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(l, r)| values_equal(l, r))
        .count()
}

/// Find the length of the common suffix (identical trailing elements)
/// The suffix must not overlap with the prefix
fn find_common_suffix(left: &[Value], right: &[Value], prefix_len: usize) -> usize {
    let left_remaining = left.len().saturating_sub(prefix_len);
    let right_remaining = right.len().saturating_sub(prefix_len);
    let max_suffix = left_remaining.min(right_remaining);

    let mut suffix_len = 0;
    for i in 0..max_suffix {
        let left_idx = left.len() - 1 - i;
        let right_idx = right.len() - 1 - i;
        if values_equal(&left[left_idx], &right[right_idx]) {
            suffix_len += 1;
        } else {
            break;
        }
    }
    suffix_len
}
