# Ordered Array Comparison Optimization

## Problem Statement

Ordered array comparison (the default mode) is now the **slowest** array comparison mode, despite being the most commonly used.

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

## Next Steps

- [ ] Investigate root cause (Myers diff algorithm complexity? Implementation details?)
- [ ] Explore optimization strategies
- [ ] Benchmark against other JSON diff tools for comparison
