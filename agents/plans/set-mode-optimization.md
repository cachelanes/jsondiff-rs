# Set Mode Array Comparison Optimization

## Problem

Benchmark results show set mode (`-s`) has O(n²) performance characteristics:

| Array Size | Set Mode Time | Ordered Mode Time | Ratio |
|------------|---------------|-------------------|-------|
| 10 elements | 36 µs | 3.2 µs | 11x |
| 100 elements | 4.0 ms | 122 µs | 33x |
| 500 elements | 99 ms | 1.3 ms | 76x |
| 1000 elements | 385 ms | 4.6 ms | 84x |

For 1000 elements, set mode is **84x slower** than ordered mode. This is unusable for large arrays.

## Root Cause Analysis

Looking at `src/diff/array.rs`, set mode likely uses nested iteration with deep equality checks:

```rust
// Suspected O(n²) pattern:
for item in left {
    if !right.iter().any(|r| values_equal(item, r)) {
        // item is removed
    }
}
```

Each `values_equal()` call is O(1) for primitives but O(m) for nested objects, making worst case O(n² * m).

## Proposed Solutions

### 1. Hash-based Set Comparison

Use a hash map to track elements:

```rust
use std::collections::HashMap;

fn diff_arrays_as_set(left: &[Value], right: &[Value], ...) -> Vec<DiffOp> {
    // Hash all values in right array
    let right_set: HashMap<u64, &Value> = right.iter()
        .map(|v| (hash_value(v), v))
        .collect();

    // O(n) lookup instead of O(n) scan
    for item in left {
        let h = hash_value(item);
        if !right_set.contains_key(&h) {
            // Removed
        }
    }
    // ... similar for added items
}
```

**Complexity:** O(n) average case
**Risk:** Hash collisions for complex nested values

### 2. Sort + Linear Scan

Sort both arrays, then do a merge-style comparison:

```rust
fn diff_arrays_as_set(left: &[Value], right: &[Value], ...) -> Vec<DiffOp> {
    let mut left_sorted: Vec<_> = left.iter().collect();
    let mut right_sorted: Vec<_> = right.iter().collect();

    left_sorted.sort_by(|a, b| compare_values(a, b));
    right_sorted.sort_by(|a, b| compare_values(a, b));

    // Merge-style O(n) comparison
    let mut i = 0;
    let mut j = 0;
    while i < left_sorted.len() && j < right_sorted.len() {
        match compare_values(left_sorted[i], right_sorted[j]) {
            Ordering::Less => { /* removed */ i += 1; }
            Ordering::Greater => { /* added */ j += 1; }
            Ordering::Equal => { i += 1; j += 1; }
        }
    }
}
```

**Complexity:** O(n log n)
**Risk:** Need total ordering for JSON values (tricky for objects)

### 3. Hybrid Approach

- For primitive arrays: Use hash-based O(n)
- For object arrays: Use current O(n²) but with early termination optimizations

```rust
fn diff_arrays_as_set(left: &[Value], right: &[Value], ...) -> Vec<DiffOp> {
    if all_primitives(left) && all_primitives(right) {
        diff_primitive_set(left, right, ...)
    } else {
        diff_complex_set(left, right, ...)  // Current implementation
    }
}
```

**Complexity:** O(n) for primitives, O(n²) for objects
**Risk:** Inconsistent performance depending on data

## Recommendation

**Option 1 (Hash-based)** is the best balance:
- Massive speedup for the common case
- Handles collisions gracefully with equality fallback
- Consistent O(n) performance

## Implementation Plan

1. Implement `hash_value()` function for sonic_rs Values
2. Replace linear scan with HashMap lookup in `diff_arrays_as_set`
3. Handle hash collisions with equality check fallback
4. Add benchmark for hash collision edge cases
5. Update documentation with new complexity guarantees

## Expected Results

| Array Size | Current | After Optimization |
|------------|---------|-------------------|
| 100 elements | 4.0 ms | ~100 µs |
| 1000 elements | 385 ms | ~1 ms |

~100-400x improvement for large arrays.

## Benchmark Command

```bash
cargo bench -- diff_arrays/set
```
