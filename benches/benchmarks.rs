use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sonic_rs::Value;

use jsondiff::diff::engine::DiffEngine;
use jsondiff::diff::types::{ArrayCompareMode, DiffConfig};

// ============================================================================
// Data Generators
// ============================================================================

/// Generate a JSON object with n keys
fn generate_object(n: usize) -> String {
    let mut parts = Vec::with_capacity(n);
    for i in 0..n {
        parts.push(format!(r#""key_{}": "value_{}""#, i, i));
    }
    format!("{{{}}}", parts.join(", "))
}

/// Generate a JSON array with n elements
fn generate_array(n: usize) -> String {
    let elements: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    format!("[{}]", elements.join(", "))
}

/// Generate a nested JSON structure with specified depth and breadth
fn generate_nested(depth: usize, breadth: usize) -> String {
    if depth == 0 {
        return r#""leaf""#.to_string();
    }

    let mut parts = Vec::with_capacity(breadth);
    for i in 0..breadth {
        parts.push(format!(r#""child_{}": {}"#, i, generate_nested(depth - 1, breadth)));
    }
    format!("{{{}}}", parts.join(", "))
}

/// Generate two objects with specified percentage of differences
fn generate_diff_pair(n: usize, diff_percent: usize) -> (String, String) {
    let diff_count = (n * diff_percent) / 100;

    let mut parts1 = Vec::with_capacity(n);
    let mut parts2 = Vec::with_capacity(n);

    for i in 0..n {
        parts1.push(format!(r#""key_{}": "value_{}""#, i, i));
        if i < diff_count {
            parts2.push(format!(r#""key_{}": "changed_{}""#, i, i));
        } else {
            parts2.push(format!(r#""key_{}": "value_{}""#, i, i));
        }
    }

    (format!("{{{}}}", parts1.join(", ")), format!("{{{}}}", parts2.join(", ")))
}

/// Generate two completely different objects (worst case)
fn generate_completely_different(n: usize) -> (String, String) {
    let mut parts1 = Vec::with_capacity(n);
    let mut parts2 = Vec::with_capacity(n);

    for i in 0..n {
        parts1.push(format!(r#""old_key_{}": "old_value_{}""#, i, i));
        parts2.push(format!(r#""new_key_{}": "new_value_{}""#, i, i));
    }

    (format!("{{{}}}", parts1.join(", ")), format!("{{{}}}", parts2.join(", ")))
}

// ============================================================================
// Object Diff Benchmarks
// ============================================================================

fn bench_diff_objects(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_objects");

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let engine = DiffEngine::new(config);

    // Benchmark identical objects (best case - tests short-circuit)
    for size in [10, 100, 1000].iter() {
        let json = generate_object(*size);
        let left: Value = sonic_rs::from_str(&json).unwrap();
        let right: Value = sonic_rs::from_str(&json).unwrap();

        group.bench_with_input(
            BenchmarkId::new("identical", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    // Benchmark objects with 10% differences
    for size in [10, 100, 1000].iter() {
        let (json1, json2) = generate_diff_pair(*size, 10);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("10pct_diff", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    // Benchmark objects with 50% differences
    for size in [10, 100, 1000].iter() {
        let (json1, json2) = generate_diff_pair(*size, 50);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("50pct_diff", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    // Benchmark completely different objects (worst case)
    for size in [10, 100, 1000].iter() {
        let (json1, json2) = generate_completely_different(*size);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("100pct_diff_worst", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Array Diff Benchmarks
// ============================================================================

fn bench_diff_arrays(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_arrays");

    // Ordered array comparison
    let ordered_config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let ordered_engine = DiffEngine::new(ordered_config);

    // Set array comparison
    let set_config = DiffConfig {
        array_mode: ArrayCompareMode::Set,
        ordered_objects: false,
    };
    let set_engine = DiffEngine::new(set_config);

    // MultiSet array comparison
    let multiset_config = DiffConfig {
        array_mode: ArrayCompareMode::MultiSet,
        ordered_objects: false,
    };
    let multiset_engine = DiffEngine::new(multiset_config);

    // Benchmark ordered array diff (with 10% elements changed)
    for size in [10, 100, 500, 1000].iter() {
        let json1 = generate_array(*size);
        let json2 = {
            let elements: Vec<String> = (0..*size)
                .map(|i| if i % 10 == 0 { (i + 1000).to_string() } else { i.to_string() })
                .collect();
            format!("[{}]", elements.join(", "))
        };

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("ordered", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| ordered_engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    // Benchmark set mode with actual differences (some elements only in one side)
    for size in [10, 100, 500, 1000].iter() {
        // First array: 0..size
        // Second array: size/2..size*3/2 (50% overlap)
        let elements1: Vec<String> = (0..*size).map(|i| i.to_string()).collect();
        let elements2: Vec<String> = (size / 2..size + size / 2).map(|i| i.to_string()).collect();

        let json1 = format!("[{}]", elements1.join(", "));
        let json2 = format!("[{}]", elements2.join(", "));

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("set_50pct_overlap", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| set_engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    // Benchmark multiset mode with duplicates
    for size in [10, 100, 500, 1000].iter() {
        // Arrays with duplicates and different counts
        let elements1: Vec<String> = (0..*size).map(|i| (i % 10).to_string()).collect();
        let elements2: Vec<String> = (0..*size).map(|i| ((i + 1) % 10).to_string()).collect();

        let json1 = format!("[{}]", elements1.join(", "));
        let json2 = format!("[{}]", elements2.join(", "));

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("multiset_with_dupes", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| multiset_engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Nested Structure Diff Benchmarks
// ============================================================================

fn bench_nested_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_nested");

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let engine = DiffEngine::new(config);

    // Benchmark nested structure diff
    for depth in [2, 4, 6].iter() {
        let json1 = generate_nested(*depth, 3);
        let json2 = generate_nested(*depth, 3).replace("leaf", "changed");

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("depth", depth),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_diff_objects,
    bench_diff_arrays,
    bench_nested_diff,
);

criterion_main!(benches);
