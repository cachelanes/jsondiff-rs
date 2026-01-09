# Deterministic Output Ordering - Alternatives Analysis

## Problem

Snapshot tests are failing because the diff output order is non-deterministic. The `diff_objects` function in `src/diff/object.rs` uses `HashSet` for key operations, which has non-deterministic iteration order.

**Failing tests:**
- `snapshot_nested_diff_pretty`
- `snapshot_nested_diff_json`

## Alternatives

### 1. Sort keys before iterating

- **How**: Collect HashSet into Vec, sort, then iterate
- **Complexity**: O(n log n) per object comparison
- **Pros**: Simple, minimal code change
- **Cons**: Repeated sorting at every object level

```rust
let mut removed_keys: Vec<&str> = left_keys.difference(&right_keys).copied().collect();
removed_keys.sort();
for key in removed_keys { ... }
```

### 2. Use BTreeSet instead of HashSet

- **How**: Replace `HashSet<&str>` with `BTreeSet<&str>`
- **Complexity**: O(log n) insertions instead of O(1), but iteration is already O(n) and sorted
- **Pros**: Automatically maintains sorted order, cleaner code
- **Cons**: Slightly slower insertions (~8-12ns vs ~1ns per lookup according to [benchmarks](https://github.com/ssomers/rust_bench_sets_compared))

```rust
use std::collections::BTreeSet;

let left_keys: BTreeSet<&str> = left.iter().map(|(k, _)| k).collect();
let right_keys: BTreeSet<&str> = right.iter().map(|(k, _)| k).collect();

// Iteration is now automatically sorted
for key in left_keys.difference(&right_keys) { ... }
```

### 3. Preserve source JSON key order

- **How**: Iterate over `left.iter()` directly instead of using sets
- **Complexity**: O(n) - no extra data structures
- **Pros**: Fastest option, output follows input structure
- **Cons**: Order depends on how JSON was written/parsed, not alphabetical

```rust
// Process keys in source order
for (key, left_val) in left.iter() {
    if let Some(right_val) = right.get(key) {
        // Common key - compare values
    } else {
        // Removed key
    }
}
for (key, right_val) in right.iter() {
    if left.get(key).is_none() {
        // Added key
    }
}
```

### 4. Sort final output operations only

- **How**: Collect all `DiffOp`s, sort by path at the end before outputting
- **Complexity**: O(m log m) where m = total operations (typically small)
- **Pros**: One-time sort, not per-object
- **Cons**: More complex implementation, need custom sort for paths

```rust
// In DiffEngine::diff()
let mut operations = diff_values(left, right, &path, &self.config);
operations.sort_by(|a, b| a.path().cmp(b.path()));
```

### 5. Use IndexMap (insertion-order preserving)

- **How**: Use `indexmap::IndexSet` which preserves insertion order
- **Complexity**: O(1) like HashSet
- **Pros**: Deterministic (follows source order)
- **Cons**: Adds a dependency, output order varies based on input

## How jd Handles This

Based on [jd's source](https://github.com/josephburnett/jd), it processes keys in the order they appear in the JSON and produces deterministic diffs by maintaining consistent traversal order.

## Performance Comparison

| Approach | Performance Impact | Code Complexity |
|----------|-------------------|-----------------|
| Sort keys | Medium - O(n log n) per object | Simple |
| **BTreeSet** | **Low - ~10ns overhead per key** | **Simple** |
| Source order | None | Medium |
| Sort final output | Low - one-time O(m log m) | Complex |
| IndexMap | None | Simple (new dep) |

## Recommendation

**Use BTreeSet** - it has minimal performance impact for typical JSON (which has small objects with few keys), and gives us automatically sorted, deterministic output with cleaner code.

For JSON objects with <100 keys (typical), the difference between HashSet and BTreeSet is negligible (~microseconds).

## Implementation

Change `src/diff/object.rs`:

```rust
// Before
use std::collections::HashSet;
let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k).collect();

// After
use std::collections::BTreeSet;
let left_keys: BTreeSet<&str> = left.iter().map(|(k, _)| k).collect();
```

This single change (HashSet -> BTreeSet) will make all iterations deterministic with minimal code changes.
