# Performance Tracking Infrastructure Plan

## Overview

This plan outlines the infrastructure needed to track performance consistently across iterations of jsondiff-rs. The goal is to detect regressions early, visualize trends, and ensure benchmarks are reproducible.

---

## Current State

### What Exists
- **Criterion benchmarks** in `benches/benchmarks.rs` covering objects, arrays, and nested structures
- **HTML reports** generated to `target/criterion/` with historical comparison
- **Manual result archives** in `agents/assets/perf-ordered-array-benchmarks/`
- **SIMD optimization** via `.cargo/config.toml` (`-C target-cpu=native`)
- **Release profile** with LTO, single codegen unit, panic=abort

### What's Missing
- No CI/CD benchmark integration
- No automated regression detection
- No visual dashboard/trending
- No noise reduction/sandboxing
- Benchmark scenarios limited to synthetic data

---

## Part 1: Sandboxing & Noise Reduction

### Problem
Benchmark results vary due to:
- CPU frequency scaling (turbo boost, thermal throttling)
- Background processes competing for resources
- NUMA effects and cache state
- Filesystem caching for I/O-bound tests

### Proposed Solutions

#### 1.1 CPU Isolation (Linux)
```bash
# Isolate CPU cores 2-3 for benchmarks (add to kernel boot params)
isolcpus=2,3 nohz_full=2,3 rcu_nocbs=2,3

# Pin benchmark process to isolated cores
taskset -c 2,3 cargo bench
```

**Trade-off**: Requires system configuration, not portable across machines.

#### 1.2 CPU Frequency Pinning
```bash
# Disable turbo boost
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# Or set performance governor
sudo cpupower frequency-set -g performance
```

**Trade-off**: Requires root access, results differ from real-world (where turbo is enabled).

#### 1.3 Warm-up & Statistical Rigor (Recommended First Step)
Criterion already handles this, but we can tune:

```rust
// In benches/benchmarks.rs
criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(5))      // Longer warm-up
        .measurement_time(Duration::from_secs(10)) // More samples
        .sample_size(200)                          // More iterations
        .noise_threshold(0.03)                     // 3% noise tolerance
        .significance_level(0.01);                 // Stricter significance
    targets = bench_diff_objects, bench_diff_arrays, bench_nested_diff
}
```

#### 1.4 Benchmark Runner Script
Create a script that applies best-effort sandboxing:

```bash
#!/bin/bash
# scripts/bench.sh

set -e

# Best-effort noise reduction
sync                                    # Flush filesystem buffers
echo 3 | sudo tee /proc/sys/vm/drop_caches 2>/dev/null || true  # Clear caches
sleep 2                                 # Let system settle

# Run with nice priority
nice -n -20 cargo bench "$@" 2>/dev/null || cargo bench "$@"
```

#### 1.5 Containerized Benchmarking (CI)
For CI, use a dedicated runner with:
- Fixed CPU count (no autoscaling)
- No parallel jobs
- Memory limits to prevent swapping

```yaml
# .github/workflows/bench.yml
jobs:
  benchmark:
    runs-on: ubuntu-latest
    # Or self-hosted runner with isolated cores
```

### Recommendation
Start with **1.3 (Criterion tuning)** and **1.4 (runner script)** as they require no system changes. Add CPU isolation later if noise remains problematic.

---

## Part 2: Performance Tracking

### 2.1 Code-Based Regression Detection

#### Option A: Criterion Baseline Comparison (Built-in)
Criterion already compares against the last run and reports regressions:
```
Performance has regressed.
  time: [1.2345 ms 1.2456 ms 1.2567 ms]
  change: [+15.234% +16.789% +18.234%] (p < 0.05)
```

**Limitation**: Only compares to previous run, not a fixed baseline.

#### Option B: `criterion-compare` for CI
Use saved baselines and fail CI on regression:

```yaml
# In CI workflow
- name: Run benchmarks
  run: cargo bench -- --save-baseline pr-${{ github.sha }}

- name: Compare to main
  run: |
    cargo bench -- --load-baseline main --save-baseline pr-${{ github.sha }}
    # Check for regressions > 10%
```

#### Option C: Custom Regression Test (Recommended)
Add a test that fails if performance exceeds thresholds:

```rust
// tests/perf_regression.rs
#[test]
#[ignore] // Run with: cargo test --release -- --ignored
fn perf_regression_array_1000_identical() {
    let (old, new) = generate_identical_arrays(1000);

    let start = Instant::now();
    for _ in 0..1000 {
        diff(&old, &new, &DiffConfig::default());
    }
    let elapsed = start.elapsed();

    let per_op = elapsed / 1000;
    let threshold = Duration::from_micros(50); // 50µs threshold

    assert!(
        per_op < threshold,
        "Performance regression: {:?} > {:?} threshold",
        per_op, threshold
    );
}
```

**Benefits**:
- Explicit thresholds checked in code
- Fails CI on regression
- No external tooling needed

### 2.2 Visual Tracking & Dashboard

#### Option A: GitHub Pages + Criterion HTML Reports
Publish Criterion's HTML reports to GitHub Pages:

```yaml
# .github/workflows/bench.yml
- name: Run benchmarks
  run: cargo bench

- name: Deploy to GitHub Pages
  uses: peaceiris/actions-gh-pages@v3
  with:
    github_token: ${{ secrets.GITHUB_TOKEN }}
    publish_dir: ./target/criterion
    destination_dir: bench/${{ github.sha }}
```

**Result**: Historical reports at `https://<user>.github.io/jsondiff-rs/bench/<sha>/`

#### Option B: Bencher.dev (SaaS)
Third-party service for benchmark tracking:

```yaml
- uses: bencherdev/bencher@main
  with:
    project: jsondiff-rs
    token: ${{ secrets.BENCHER_API_TOKEN }}
    adapter: rust_criterion
```

**Benefits**: Automatic trending, alerts, PR comments
**Drawback**: External dependency, possible cost

#### Option C: Custom JSON + Chart.js Dashboard (Recommended)
1. Export Criterion results to JSON
2. Aggregate into a history file
3. Render with a simple HTML + Chart.js page

```bash
# After cargo bench, extract results
jq -s '.' target/criterion/*/new/estimates.json > bench-results.json
```

```html
<!-- docs/bench/index.html -->
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
<canvas id="perfChart"></canvas>
<script>
  fetch('history.json')
    .then(r => r.json())
    .then(data => {
      new Chart(document.getElementById('perfChart'), {
        type: 'line',
        data: {
          labels: data.map(d => d.commit.slice(0,7)),
          datasets: [{
            label: 'array_1000_identical (µs)',
            data: data.map(d => d.benchmarks['array_1000_identical'])
          }]
        }
      });
    });
</script>
```

#### Option D: GitHub Action Bot Comments
Post benchmark comparison as PR comments:

```yaml
- uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'cargo'
    output-file-path: target/criterion/**/new/estimates.json
    github-token: ${{ secrets.GITHUB_TOKEN }}
    comment-on-alert: true
    alert-threshold: '150%'  # Alert if 50% slower
```

### Recommendation
Combine:
1. **Option C (regression tests)** for CI gating
2. **Option A or C (GitHub Pages dashboard)** for visualization
3. **Option D (PR comments)** for developer visibility

---

## Part 3: Benchmark Scenario Improvements (Future)

### Current Coverage
| Category | Sizes | Scenarios |
|----------|-------|-----------|
| Objects | 10, 100, 1000 | identical, 10%, 50%, 100% diff |
| Arrays (ordered) | 10, 100, 500, 1000 | identical, 10%, 50% end-only |
| Arrays (set/multiset) | 10, 100, 500, 1000 | 50% overlap, duplicates |
| Nested | depth 2,4,6 × breadth 3 | identical, modified |

### Gaps to Address

#### 3.1 Real-World JSON Samples
- **Package.json** (npm metadata, moderate nesting)
- **OpenAPI specs** (deep nesting, many keys)
- **GeoJSON** (large coordinate arrays)
- **Log entries** (flat, many records)

Sources:
- https://github.com/jdorfman/awesome-json-datasets
- Generate from public APIs

#### 3.2 Scale Testing
- **Large files**: 1MB, 10MB, 100MB JSON
- **Wide objects**: 10k, 100k keys
- **Deep nesting**: depth 10, 20, 50
- **Long arrays**: 10k, 100k, 1M elements

#### 3.3 Edge Cases
- Unicode keys/values
- Numeric precision edge cases
- Empty objects/arrays at various positions
- Null vs missing key distinction

#### 3.4 Comparative Benchmarks
Compare against other tools:
- `jd` (Go)
- `json-diff` (Node.js)
- `jq` (C)
- `diff` on pretty-printed JSON

---

## Implementation Phases

### Phase 1: Foundation (Immediate)
1. Tune Criterion configuration for lower noise
2. Create `scripts/bench.sh` runner script
3. Add 3-5 regression threshold tests
4. Set up basic GitHub Actions workflow for benchmarks

### Phase 2: Visibility (Short-term)
1. Publish Criterion reports to GitHub Pages
2. Add PR comment bot for benchmark comparison
3. Create simple trending dashboard

### Phase 3: Comprehensive (Medium-term)
1. Add real-world JSON test fixtures
2. Expand size ranges for scale testing
3. Add comparative benchmarks against other tools
4. Consider self-hosted runner with CPU isolation

### Phase 4: Polish (Long-term)
1. Memory profiling integration
2. Flamegraph generation in CI
3. Cross-platform benchmarking (Linux, macOS, Windows)
4. Public benchmark leaderboard

---

## File Structure After Implementation

```
jsondiff-rs/
├── .github/workflows/
│   └── bench.yml              # Benchmark CI workflow
├── benches/
│   ├── benchmarks.rs          # Main benchmarks (tuned)
│   ├── regression.rs          # Threshold-based regression tests
│   └── fixtures/              # Real-world JSON samples
│       ├── package.json
│       ├── openapi-spec.json
│       └── geojson-sample.json
├── scripts/
│   └── bench.sh               # Sandboxed benchmark runner
├── docs/
│   └── bench/
│       ├── index.html         # Dashboard
│       └── history.json       # Historical data
└── target/criterion/          # Generated reports (gitignored)
```

---

## Decision Points for Discussion

1. **CI runner**: Use GitHub-hosted or self-hosted for benchmarks?
2. **Regression thresholds**: What % degradation should fail CI?
3. **Dashboard hosting**: GitHub Pages, external service, or none?
4. **Real-world samples**: Which domains to prioritize?
5. **Comparative benchmarks**: Which tools to compare against?

---

## References

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Bencher.dev Documentation](https://bencher.dev/docs/)
- [GitHub Action Benchmark](https://github.com/benchmark-action/github-action-benchmark)
- [Linux CPU Isolation](https://www.kernel.org/doc/html/latest/admin-guide/kernel-parameters.html)
