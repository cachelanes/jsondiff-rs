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
    // =========================================================================
    // Phase 1: Early termination paths
    // =========================================================================

    // Fast path: both arrays empty
    if left.is_empty() && right.is_empty() {
        return vec![];
    }

    // Fast path: left array empty - all elements are additions
    if left.is_empty() {
        return right
            .iter()
            .enumerate()
            .map(|(i, val)| DiffOp::Added {
                path: path.append_index(i),
                value: val.clone(),
            })
            .collect();
    }

    // Fast path: right array empty - all elements are removals
    if right.is_empty() {
        return left
            .iter()
            .enumerate()
            .map(|(i, val)| DiffOp::Removed {
                path: path.append_index(i),
                value: val.clone(),
            })
            .collect();
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
            ops.extend(diff_values(&left[i], &right[i], &child_path, config));
        }
        return ops;
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
        ops.extend(diff_values(&left[i], &right[i], &child_path, config));
    }

    // Process the middle portion with Myers diff
    // Phase 3: Use PrehashedWrapper to avoid repeated hash computation
    // Note: Phase 4 (small array fast path) was tested but prefix/suffix matching
    // already handles the small array case effectively
    if !left_middle.is_empty() || !right_middle.is_empty() {
        let left_wrapped: Vec<PrehashedWrapper> = left_middle.iter().map(PrehashedWrapper::new).collect();
        let right_wrapped: Vec<PrehashedWrapper> = right_middle.iter().map(PrehashedWrapper::new).collect();

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
                    for i in 0..len {
                        // Adjust indices for prefix offset
                        let actual_left_idx = prefix_len + old_index + i;
                        let actual_right_idx = prefix_len + new_index + i;
                        let child_path = path.append_index(actual_right_idx);
                        ops.extend(diff_values(
                            &left[actual_left_idx],
                            &right[actual_right_idx],
                            &child_path,
                            config,
                        ));
                    }
                }
                similar::DiffOp::Delete {
                    old_index, old_len, ..
                } => {
                    for i in 0..old_len {
                        let actual_idx = prefix_len + old_index + i;
                        ops.push(DiffOp::Removed {
                            path: path.append_index(actual_idx),
                            value: left[actual_idx].clone(),
                        });
                    }
                }
                similar::DiffOp::Insert {
                    new_index, new_len, ..
                } => {
                    for i in 0..new_len {
                        let actual_idx = prefix_len + new_index + i;
                        ops.push(DiffOp::Added {
                            path: path.append_index(actual_idx),
                            value: right[actual_idx].clone(),
                        });
                    }
                }
                similar::DiffOp::Replace {
                    old_index,
                    old_len,
                    new_index,
                    new_len,
                } => {
                    let min_len = old_len.min(new_len);
                    for i in 0..min_len {
                        let actual_left_idx = prefix_len + old_index + i;
                        let actual_right_idx = prefix_len + new_index + i;
                        ops.push(DiffOp::Modified {
                            path: path.append_index(actual_right_idx),
                            old_value: left[actual_left_idx].clone(),
                            new_value: right[actual_right_idx].clone(),
                        });
                    }
                    for i in min_len..old_len {
                        let actual_idx = prefix_len + old_index + i;
                        ops.push(DiffOp::Removed {
                            path: path.append_index(actual_idx),
                            value: left[actual_idx].clone(),
                        });
                    }
                    for i in min_len..new_len {
                        let actual_idx = prefix_len + new_index + i;
                        ops.push(DiffOp::Added {
                            path: path.append_index(actual_idx),
                            value: right[actual_idx].clone(),
                        });
                    }
                }
            }
        }
    }

    // Process suffix: check for deep differences
    for i in 0..suffix_len {
        let left_idx = left.len() - suffix_len + i;
        let right_idx = right.len() - suffix_len + i;
        let child_path = path.append_index(right_idx);
        ops.extend(diff_values(&left[left_idx], &right[right_idx], &child_path, config));
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
/// Note: Kept for potential external use, but ordered mode now uses PrehashedWrapper
#[allow(dead_code)]
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

// ============================================================================
// Phase 3: PrehashedWrapper for hash acceleration
// ============================================================================

/// Wrapper that stores a pre-computed hash with the value
/// This avoids repeated hash computation during Myers diff comparisons
#[derive(Clone)]
struct PrehashedWrapper<'a> {
    value: &'a Value,
    hash: u64,
}

impl<'a> PrehashedWrapper<'a> {
    fn new(value: &'a Value) -> Self {
        PrehashedWrapper {
            value,
            hash: compute_value_hash(value),
        }
    }
}

impl<'a> PartialEq for PrehashedWrapper<'a> {
    fn eq(&self, other: &Self) -> bool {
        // Fast path: if hashes differ, values definitely differ
        if self.hash != other.hash {
            return false;
        }
        // Hashes match, need to verify with deep comparison (handles collisions)
        values_equal(self.value, other.value)
    }
}

impl<'a> Eq for PrehashedWrapper<'a> {}

impl<'a> Hash for PrehashedWrapper<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Just use the pre-computed hash
        self.hash.hash(state);
    }
}

impl<'a> PartialOrd for PrehashedWrapper<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for PrehashedWrapper<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Use the pre-computed hash for ordering
        self.hash.cmp(&other.hash)
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
