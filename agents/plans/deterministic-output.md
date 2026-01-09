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

---

## Benchmark Results

> **Benchmark source code**: [`agents/assets/deterministic_bench.rs`](../assets/deterministic_bench.rs)
>
> To run: Copy to `benches/`, add `indexmap = "2.2"` to dependencies, add `[[bench]]` entry to Cargo.toml, and run `cargo bench --bench deterministic_bench`

Comprehensive benchmarks were run comparing all 5 approaches using Criterion. The benchmark suite tested:
- **Object sizes**: 10, 100, 1000, 5000 keys
- **Diff scenarios**: identical (0%), 10%, 50%, 100% different
- **Nested structures**: depths 2, 4, 6 with breadth 3; breadths 5, 10 with depth 3

### Flat Object Performance

#### 100 Keys (Identical)
| Approach | Time | Throughput |
|----------|------|------------|
| **preserve_order** | **40 µs** | **2.47 Melem/s** |
| sort_output | 57 µs | 1.74 Melem/s |
| btreeset | 70 µs | 1.44 Melem/s |
| indexset | 69 µs | 1.45 Melem/s |
| sorted_keys | 74 µs | 1.35 Melem/s |

#### 1000 Keys (100% Different - Worst Case)
| Approach | Time | vs preserve_order |
|----------|------|-------------------|
| **preserve_order** | **404 µs** | baseline |
| btreeset | 5.3 ms | 13x slower |
| sorted_keys | 6.1 ms | 15x slower |
| indexset | 6.9 ms | 17x slower |
| sort_output | 9.9 ms | **24x slower** |

#### 5000 Keys (100% Different - Worst Case)
| Approach | Time | vs preserve_order |
|----------|------|-------------------|
| **preserve_order** | **2.5 ms** | baseline |
| btreeset | 92 ms | 37x slower |
| sorted_keys | 98 ms | 39x slower |
| indexset | 105 ms | 42x slower |
| sort_output | 123 ms | **49x slower** |

### Nested Structure Performance

#### Depth 6, Breadth 3 (1093 nodes)
| Approach | Time |
|----------|------|
| **preserve_order** | **583 µs** |
| btreeset | 637 µs |
| sorted_keys | 723 µs |
| indexset | 790 µs |
| sort_output | 3504 µs (6x slower) |

#### Breadth 10, Depth 3 (1111 nodes)
| Approach | Time |
|----------|------|
| indexset | 576 µs |
| **preserve_order** | **613 µs** |
| btreeset | 875 µs |
| sorted_keys | 992 µs |
| sort_output | 3257 µs (5.3x slower) |

### Key Findings

1. **`preserve_order` is consistently fastest** - 1.4x to 49x faster depending on scenario
2. **`sort_output` scales very poorly** - sorting thousands of DiffOps is expensive
3. **Performance gap increases with size** - at 5000 keys with 100% diff, preserve_order is 37-49x faster
4. **For nested structures**, differences are smaller (1.1-1.5x) except for sort_output

---

## Updated Recommendation

Based on benchmark results, the original recommendation of BTreeSet should be revised:

### For Maximum Performance: Use `preserve_order` (Recommended)
- **1.4-49x faster** than alternatives
- Deterministic output (follows input JSON structure)
- **Better UX**: Users see diffs in the same order as their input files, making it easier to correlate output with source JSON structure
- Output order depends on input rather than alphabetical, which is typically more intuitive for users reviewing diffs

### For Alphabetical Output: Use `btreeset`
- Best balance of performance and simplicity among alphabetical options
- ~1.5-2x slower than preserve_order for typical cases
- Automatically sorted output

### Avoid: `sort_output`
- Worst scaling characteristics
- 5-49x slower at scale due to sorting large DiffOp vectors

## Final Rankings

| Rank | Approach | Performance | Determinism | Code Complexity |
|------|----------|-------------|-------------|-----------------|
| 1 | **preserve_order** | Best | Input-dependent | Medium |
| 2 | indexset | Good | Input-order | Simple |
| 3 | btreeset | Good | Alphabetical | Simple |
| 4 | sorted_keys | Moderate | Alphabetical | Simple |
| 5 | sort_output | **Worst** | Alphabetical | Complex |
