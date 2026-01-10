# Ordered Array Comparison Optimization

## Problem Statement

Ordered array comparison (the default mode) is now the **slowest** array comparison mode, despite being the most commonly used.

**This violates core project requirements:**
- Primary requirement: "Focus on speed: be lightning fast" (requirements.md)
- Primary requirement: "Arrays should be considered ordered by default"
- The default user experience should be the fastest, not the slowest

### Current Benchmark Results

| Size          | Ordered (Myers)   | Set Mode   | Multiset Mode   |
|---------------|-------------------|------------|-----------------|
| 10 elements   | 2.7 µs            | 25 µs      | 45 µs           |
| 100 elements  | 93 µs             | 76 µs      | 110 µs          |
| 500 elements  | 1.0 ms            | 421 µs     | 520 µs          |
| 1000 elements | 3.0 ms            | 806 µs     | 1.1 ms          |

### Key Observations

1. **Ordered mode is slower than set/multiset for large arrays** - At 1000 elements, ordered mode (3.0 ms) is ~3.7x slower than set mode (806 µs)

2. **Performance gap widens with size** - At 100 elements ordered is comparable, but by 1000 elements it's significantly slower

3. **This is the default mode** - Users who don't specify `-s` or `-m` get the slowest performance

### Impact

- Default user experience is suboptimal
- Large JSON arrays (common in API responses, logs, data exports) will be slow to diff
- Users may not realize switching to set/multiset mode could be faster

## Root Cause Analysis

After comprehensive investigation of `src/diff/array.rs`:

1. **No early termination**: Every comparison runs through Myers, even identical arrays
2. **No prefix/suffix optimization**: Common leading/trailing elements not skipped
3. **Repeated hashing**: `ValueWrapper` computes hashes on every `eq()` call during Myers algorithm
4. **No adaptive sizing**: Set/multiset use linear search for small arrays (<20), ordered doesn't
5. **Myers O(N×M×D) complexity**: When edit distance D is large, performance degrades to quadratic

### Comparison with Set/Multiset

Set and multiset modes are faster because they:
- Use hash-based lookups: O(N+M) average case
- Have adaptive thresholds: linear search for small arrays, HashMap for large
- Pre-compute hashes once and cache them
- Skip order-preserving diff logic entirely

## Optimization Strategy

We'll implement **5 phases of optimization** to make ordered mode as fast as possible:

### Phase 1: Early Termination Paths ✓

Add fast paths for common cases:
- Both arrays empty → return immediately
- One array empty → all adds or all removes
- Arrays identical (hash check + verify) → return immediately

**Expected gain**: 60x for identical arrays, 10-20% for other cases

### Phase 2: Prefix/Suffix Matching ✓

Reduce problem size by finding common elements at both ends:
- Find common prefix (identical leading elements)
- Find common suffix (identical trailing elements)
- Only run Myers on the middle portion

**Expected gain**: 2-5x for high-similarity arrays (common case)

### Phase 3: Hash-Accelerated ValueWrapper ✓

Create `PrehashedWrapper` that:
- Stores pre-computed hash with each value
- Uses hash for O(1) equality fast-path
- Only does deep comparison on hash match

**Expected gain**: 1.5-2x by eliminating repeated hash computation

### Phase 4: Small Array Fast Path ✓

Add threshold-based optimization like set/multiset:
- Arrays <20 elements: use simple O(N×M) DP-based LCS (cache-friendly)
- Arrays ≥20 elements: use optimized Myers

**Expected gain**: 1.2-1.5x for small arrays

### Phase 5: Algorithm Comparison ✓

Implement **both** algorithms and benchmark to find the winner:

**Option A: Optimized similar (Myers)**
- Keep using `similar` crate with all optimizations above
- Benefit from existing integration and stability

**Option B: imara-diff (Histogram)**
- Add `imara-diff` crate with Histogram algorithm
- Reported 10-100% faster than Myers in benchmarks
- Better worst-case behavior with GNU diff heuristics
- Native interning support for pre-hashed tokens

We'll implement both, benchmark both, and keep the faster one.

## Implementation Strategy

We'll use a **hybrid approach** combining incremental implementation and synthetic benchmarking:

**Why hybrid?**
- Unlike the deterministic output problem (which had 5 independent alternatives), we have:
  - **4 cumulative optimizations** (Phases 1-4) → incremental approach is better
  - **2 algorithm alternatives** (Phase 5) → synthetic benchmark for clean comparison
- This gives us speed (don't revert cumulative improvements), safety (benchmark after each phase), and clean algorithm comparison.

### Phases 1-4: Incremental Implementation (Cumulative Optimizations)

These optimizations build on each other and should be cumulative:
- Phase 1 (early termination) → stays
- Phase 2 (prefix/suffix) → builds on Phase 1, stays
- Phase 3 (prehashing) → builds on Phases 1-2, stays
- Phase 4 (small array fast path) → builds on Phases 1-3, stays

**Process:**
```bash
# Baseline benchmark
cargo bench --bench benchmarks > results/baseline.txt

# Implement Phase 1
cargo bench --bench benchmarks > results/phase1.txt
# Verify improvement, commit

# Implement Phase 2 (on top of Phase 1)
cargo bench --bench benchmarks > results/phase2.txt
# Verify additional improvement, commit

# Continue for Phases 3-4...
```

### Phase 5: Synthetic Benchmark (Algorithm Comparison)

For comparing Myers-optimized vs Histogram, create an isolated synthetic benchmark similar to the deterministic output analysis.

**Why synthetic?**: We're comparing two completely different algorithms - isolated comparison prevents cumulative effects and gives clean data.

**Process:**
1. Create `agents/assets/ordered_array_bench.rs`
2. Implement both algorithms in isolation:
   - `diff_with_myers_optimized()` - similar crate with PrehashedWrapper
   - `diff_with_histogram()` - imara-diff
3. Benchmark against multiple scenarios:
   - Identical arrays (best case)
   - 95% similar (common case)
   - 50% similar (medium changes)
   - Completely different (worst case)
   - Small arrays (<20 elements)
   - Large arrays (1000+ elements)
4. Analyze results and choose winner
5. Integrate winner into main codebase

### Final Verification

After all phases complete:
```bash
# Full benchmark suite
cargo bench --bench benchmarks > results/final.txt

# Compare against baseline
diff results/baseline.txt results/final.txt

# Verify no regressions in set/multiset modes
# Property tests
cargo test --test property_tests

# Snapshot tests
cargo insta test
```

## Implementation Plan

### 1. Code Changes

**File: `src/diff/array.rs`** (lines 14-105)
- Add early termination checks at function start
- Add hash pre-computation: `let left_hashes: Vec<u64> = left.iter().map(compute_value_hash).collect()`
- Add prefix/suffix matching functions
- Create `PrehashedWrapper<'a>` struct with pre-computed hash
- Add `diff_small_arrays_ordered()` for arrays <20 elements
- Implement `diff_arrays_ordered_myers()` using optimized similar
- Implement `diff_arrays_ordered_histogram()` using imara-diff
- Main function selects best algorithm based on benchmarks

**File: `Cargo.toml`**
- Add dependency: `imara-diff = "0.1"`

**File: `benches/benchmarks.rs`**
- Add benchmark: identical arrays (best case)
- Add benchmark: 95% similar arrays (high similarity)
- Add benchmark: small arrays (<20 elements)
- Add benchmark: completely different arrays (worst case)

### 2. Testing Strategy

#### Correctness Testing (per requirements.md priority order)
1. **Property tests**: Ensure output semantically equivalent (highest priority)
2. **Snapshot tests**: `cargo insta test` - may need updates for different algorithm
3. **Unit tests**: `cargo test` - all must pass

#### Performance Testing
- Benchmark Myers-optimized vs Histogram on all scenarios
- Ensure **no regressions** for set/multiset modes
- Compare before/after for ordered mode:
  - 10 elements
  - 100 elements
  - 500 elements
  - 1000 elements
  - Identical arrays
  - High-similarity arrays

#### Regression Prevention
**Critical**: Verify set and multiset modes maintain performance:
```bash
# Before changes
cargo bench --bench benchmarks > before.txt

# After changes
cargo bench --bench benchmarks > after.txt

# Compare - set/multiset should not regress
diff before.txt after.txt
```

### 3. Performance Targets

| Scenario | Current | Target | Improvement |
|----------|---------|--------|-------------|
| 1000 elements, 10% changes | 3.0 ms | ~600 µs | 5x |
| 1000 elements, identical | 3.0 ms | ~50 µs | 60x |
| 1000 elements, 95% same | 2.5 ms | ~200 µs | 12x |
| 100 elements | 93 µs | ~40 µs | 2x |
| Set mode (regression check) | 806 µs | ≤806 µs | No regression |
| Multiset mode (regression check) | 1.1 ms | ≤1.1 ms | No regression |

### 4. Decision Criteria

After implementing both algorithms, choose based on:
1. **Performance**: Which is faster across all scenarios?
2. **Worst-case behavior**: How do they handle completely different arrays?
3. **Code complexity**: Maintenance burden
4. **Dependencies**: `imara-diff` is an additional dependency

If imara-diff is >20% faster across the board, use it. Otherwise stick with optimized similar.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Hash collisions cause incorrect diffs | Always verify with `values_equal()` after hash match |
| Prefix/suffix logic has off-by-one bugs | Add comprehensive property tests and edge case tests |
| Performance regression for set/multiset | Benchmark all modes before/after, no shared code changes |
| Breaking existing behavior | Full test suite must pass, snapshot tests verify output |
| imara-diff integration issues | Keep both implementations, can fallback to Myers |

## Next Steps (Ordered)

### Baseline & Setup
- [ ] Create `results/` directory for benchmark outputs
- [ ] Run baseline benchmark: `cargo bench --bench benchmarks > results/baseline.txt`

### Phases 1-4: Incremental Implementation
- [ ] **Phase 1**: Implement early termination paths
  - [ ] Benchmark: `cargo bench --bench benchmarks > results/phase1.txt`
  - [ ] Verify improvement vs baseline
  - [ ] Commit
- [ ] **Phase 2**: Implement prefix/suffix matching (on top of Phase 1)
  - [ ] Benchmark: `cargo bench --bench benchmarks > results/phase2.txt`
  - [ ] Verify improvement vs phase1
  - [ ] Commit
- [ ] **Phase 3**: Implement PrehashedWrapper (on top of Phases 1-2)
  - [ ] Benchmark: `cargo bench --bench benchmarks > results/phase3.txt`
  - [ ] Verify improvement vs phase2
  - [ ] Commit
- [ ] **Phase 4**: Implement small array fast path (on top of Phases 1-3)
  - [ ] Benchmark: `cargo bench --bench benchmarks > results/phase4.txt`
  - [ ] Verify improvement vs phase3
  - [ ] Commit

### Phase 5: Algorithm Comparison (Synthetic Benchmark)
- [ ] Create `agents/assets/ordered_array_bench.rs`
- [ ] Implement `diff_with_myers_optimized()` in isolation
- [ ] Implement `diff_with_histogram()` with imara-diff in isolation
- [ ] Add benchmark scenarios: identical, 95% similar, 50% similar, completely different
- [ ] Add benchmark scenarios: small arrays (<20), large arrays (1000+)
- [ ] Run synthetic benchmark suite
- [ ] Analyze results and choose winner
- [ ] Integrate winning algorithm into `src/diff/array.rs`

### Final Verification
- [ ] Run full benchmark suite: `cargo bench --bench benchmarks > results/final.txt`
- [ ] Compare final vs baseline: verify 5x improvement target
- [ ] **Verify no regressions**: Set mode ≤806µs, Multiset mode ≤1.1ms
- [ ] Run property tests: `cargo test --test property_tests`
- [ ] Run snapshot tests: `cargo insta test` (may need review/accept)
- [ ] Run unit tests: `cargo test`
- [ ] Update documentation with performance characteristics
