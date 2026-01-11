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
- ✅ **CodSpeed CI integration** via `.github/workflows/codspeed.yml`
- ✅ **codspeed-criterion-compat** drop-in replacement for criterion (works with both `cargo bench` and `cargo codspeed`)

### What's Been Implemented
| Component | Status | Details |
|-----------|--------|---------|
| CI benchmark workflow | ✅ Done | `.github/workflows/codspeed.yml` |
| Visual dashboard | ✅ Done | CodSpeed provides trending & graphs |
| PR comments | ✅ Done | CodSpeed posts comparison on PRs |
| Regression detection | ✅ Done | CodSpeed tracks & alerts |
| Real-world timing | ✅ Done | CodSpeed `walltime` mode (bare-metal macro runners, captures SIMD benefits) |

### What's Remaining
- Local noise reduction (Criterion tuning, runner script)
- Memory profiling integration

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

> **Status**: ✅ Largely complete via CodSpeed integration

CodSpeed now handles:
- **CI benchmarking** with instruction-count measurement (no noise)
- **Visual dashboard** with historical trending
- **PR comments** showing before/after comparison
- **Regression alerts** when performance degrades

### 2.1 Code-Based Regression Detection (Optional Enhancement)

For additional safety net beyond CodSpeed, consider explicit threshold tests:

#### Option: Custom Regression Test
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
- Fails CI on regression independently of CodSpeed
- Catches regressions even if CodSpeed is unavailable

### 2.2 Visual Tracking & Dashboard

> **Status**: ✅ Complete via CodSpeed

CodSpeed provides:
- Historical trending graphs
- Per-benchmark drill-down
- Comparison between any two commits
- PR integration with inline comments

Access the dashboard at: https://codspeed.io (after first CI run)

---

## Part 3: Benchmark Scenario Improvements ✅ Complete

### Current Coverage
| Category | Sizes | Scenarios |
|----------|-------|-----------|
| Objects | 10, 100, 1000 | identical, 10%, 50%, 100% diff |
| Arrays (ordered) | 10, 100, 500, 1000 | identical, 10%, 50% end-only |
| Arrays (set/multiset) | 10, 100, 500, 1000 | 50% overlap, duplicates |
| Nested | depth 2,4,6 × breadth 3 | identical, modified |
| Real-world GeoJSON | 6-12MB | identical, 3 cross-source diffs |
| Real-world package-lock | 680-750KB | identical, ~8%, ~18%, ~42% diffs |

### Implemented Fixtures

| File | Source | License | Size | Purpose |
|------|--------|---------|------|---------|
| `india-osm.geojson` | [datameet/maps](https://github.com/datameet/maps) | ODbL | 6.3MB | GeoJSON coordinates (OpenStreetMap) |
| `india-composite.geojson` | [datameet/maps](https://github.com/datameet/maps) | CC-0 | 11MB | GeoJSON (composite sources) |
| `india-soi.geojson` | [datameet/maps](https://github.com/datameet/maps) | CC-by-sa 2.5 | 12MB | GeoJSON (Survey of India) |
| `vscode-1.94-package-lock.json` | [microsoft/vscode](https://github.com/microsoft/vscode) | MIT | 751KB | npm lockfile baseline |
| `vscode-1.95-package-lock.json` | [microsoft/vscode](https://github.com/microsoft/vscode) | MIT | 702KB | ~18% diff from 1.94 |
| `vscode-1.100-package-lock.json` | [microsoft/vscode](https://github.com/microsoft/vscode) | MIT | 708KB | ~8% diff from 1.95 |
| `vscode-1.108-package-lock.json` | [microsoft/vscode](https://github.com/microsoft/vscode) | MIT | 681KB | ~42% diff from 1.94 |

### Future Enhancements

#### 3.1 Scale Testing
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

### Phase 1: Foundation ✅ Complete
1. ~~Set up basic GitHub Actions workflow for benchmarks~~ → CodSpeed workflow
2. ~~Add PR comment bot for benchmark comparison~~ → CodSpeed handles
3. ~~Create simple trending dashboard~~ → CodSpeed dashboard

### Phase 2: Local Development Experience (Next)
1. Tune Criterion configuration for lower noise (local runs)
2. Create `scripts/bench.sh` or `just bench` runner script
3. Add 3-5 regression threshold tests (optional safety net)

### Phase 3: Benchmark Scenario Improvements ✅ Complete
1. ~~Add real-world JSON test fixtures~~ → India GeoJSON + VSCode package-lock
2. Expand size ranges for scale testing (future)
3. Add comparative benchmarks against other tools (future)

### Phase 4: Advanced (Future)
1. Memory profiling integration (DHAT, heaptrack)
2. Flamegraph generation in CI
3. Cross-platform benchmarking (Linux, macOS, Windows)

---

## File Structure

```
jsondiff-rs/
├── .github/workflows/
│   ├── codspeed.yml           # ✅ CodSpeed benchmark CI (walltime + macro runners)
│   └── claude.yml             # Claude Code integration
├── benches/
│   ├── benchmarks.rs          # Main benchmarks (criterion-compat)
│   └── fixtures/              # Real-world JSON samples
│       ├── india-osm.geojson          # 6.3MB (datameet/maps - ODbL)
│       ├── india-composite.geojson    # 11MB (datameet/maps - CC-0)
│       ├── india-soi.geojson          # 12MB (datameet/maps - CC-by-sa)
│       ├── vscode-1.94-package-lock.json   # 751KB (microsoft/vscode - MIT)
│       ├── vscode-1.95-package-lock.json   # 702KB
│       ├── vscode-1.100-package-lock.json  # 708KB
│       ├── vscode-1.108-package-lock.json  # 681KB
│       └── LICENSES.md        # Third-party attribution
├── justfile                   # ✅ Task runner (includes bench commands)
└── target/criterion/          # Generated reports (gitignored)
```

---

## Decision Points (Remaining)

1. ~~**CI runner**: Use GitHub-hosted or self-hosted?~~ → CodSpeed macro runners (ARM64 bare-metal)
2. ~~**Dashboard hosting**: GitHub Pages, external service?~~ → CodSpeed dashboard
3. ~~**Real-world samples**: Which domains to prioritize?~~ → GeoJSON (geographic), package-lock (config)
4. **Regression thresholds**: What % degradation should CodSpeed alert on? (configurable in CodSpeed)
5. **Comparative benchmarks**: Which tools to compare against?

---

## References

- [CodSpeed Documentation](https://codspeed.io/docs/)
- [CodSpeed Criterion Integration](https://codspeed.io/docs/benchmarks/rust/criterion)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Iai-Callgrind](https://github.com/iai-callgrind/iai-callgrind) - Alternative for self-hosted instruction counting
- [rustc-perf](https://github.com/rust-lang/rustc-perf) - How Rust compiler tracks performance
- [Linux CPU Isolation](https://www.kernel.org/doc/html/latest/admin-guide/kernel-parameters.html)
