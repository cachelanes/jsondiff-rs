use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sonic_rs::Value;

// Import from the jsondiff crate
use jsondiff::diff::engine::DiffEngine;
use jsondiff::diff::types::{ArrayCompareMode, DiffConfig};
use jsondiff::parser::json::JsonParser;

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

/// Generate two similar objects with some differences
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

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_parsing");

    // Test different object sizes
    for size in [10, 100, 1000, 10000].iter() {
        let json = generate_object(*size);
        let bytes = json.as_bytes();

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("object", size),
            &json,
            |b, json| {
                b.iter(|| {
                    let _: Value = sonic_rs::from_str(black_box(json)).unwrap();
                });
            },
        );
    }

    // Test different array sizes
    for size in [10, 100, 1000, 10000].iter() {
        let json = generate_array(*size);
        let bytes = json.as_bytes();

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("array", size),
            &json,
            |b, json| {
                b.iter(|| {
                    let _: Value = sonic_rs::from_str(black_box(json)).unwrap();
                });
            },
        );
    }

    // Test nested structures
    for depth in [2, 4, 6].iter() {
        let json = generate_nested(*depth, 4);
        let bytes = json.as_bytes();

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("nested_depth", depth),
            &json,
            |b, json| {
                b.iter(|| {
                    let _: Value = sonic_rs::from_str(black_box(json)).unwrap();
                });
            },
        );
    }

    group.finish();
}

fn bench_diff_objects(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_objects");

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let engine = DiffEngine::new(config);

    // Benchmark identical objects (best case)
    for size in [10, 100, 1000].iter() {
        let json = generate_object(*size);
        let left: Value = sonic_rs::from_str(&json).unwrap();
        let right: Value = sonic_rs::from_str(&json).unwrap();

        group.bench_with_input(
            BenchmarkId::new("identical", size),
            &(left.clone(), right.clone()),
            |b, (left, right)| {
                b.iter(|| {
                    engine.diff(black_box(left), black_box(right))
                });
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
            &(left.clone(), right.clone()),
            |b, (left, right)| {
                b.iter(|| {
                    engine.diff(black_box(left), black_box(right))
                });
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
            &(left.clone(), right.clone()),
            |b, (left, right)| {
                b.iter(|| {
                    engine.diff(black_box(left), black_box(right))
                });
            },
        );
    }

    group.finish();
}

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

    // Benchmark ordered array diff
    for size in [10, 100, 500].iter() {
        let json1 = generate_array(*size);
        let json2 = {
            // Create array with some elements changed
            let elements: Vec<String> = (0..*size)
                .map(|i| if i % 10 == 0 { (i + 1000).to_string() } else { i.to_string() })
                .collect();
            format!("[{}]", elements.join(", "))
        };

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("ordered", size),
            &(left.clone(), right.clone()),
            |b, (left, right)| {
                b.iter(|| {
                    ordered_engine.diff(black_box(left), black_box(right))
                });
            },
        );
    }

    // Benchmark set array diff (same elements, different order)
    for size in [10, 100, 500].iter() {
        let elements1: Vec<String> = (0..*size).map(|i| i.to_string()).collect();
        let elements2: Vec<String> = (0..*size).rev().map(|i| i.to_string()).collect();

        let json1 = format!("[{}]", elements1.join(", "));
        let json2 = format!("[{}]", elements2.join(", "));

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("set_reversed", size),
            &(left.clone(), right.clone()),
            |b, (left, right)| {
                b.iter(|| {
                    set_engine.diff(black_box(left), black_box(right))
                });
            },
        );
    }

    group.finish();
}

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
            &(left.clone(), right.clone()),
            |b, (left, right)| {
                b.iter(|| {
                    engine.diff(black_box(left), black_box(right))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_diff_objects,
    bench_diff_arrays,
    bench_nested_diff,
);

criterion_main!(benches);
