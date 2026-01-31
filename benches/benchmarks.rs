use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sonic_rs::Value;
use std::fs;

use jsondiff::diff::engine::DiffEngine;
use jsondiff::diff::types::{ArrayCompareMode, DiffConfig, SetKeyConfig};
use std::collections::HashMap;

// Path to benchmark fixtures
const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/benches/fixtures");

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
        set_keys: None,
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
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
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
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
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
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
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
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
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
        set_keys: None,
    };
    let ordered_engine = DiffEngine::new(ordered_config);

    // Set array comparison
    let set_config = DiffConfig {
        array_mode: ArrayCompareMode::Set,
        ordered_objects: false,
        set_keys: None,
    };
    let set_engine = DiffEngine::new(set_config);

    // MultiSet array comparison
    let multiset_config = DiffConfig {
        array_mode: ArrayCompareMode::MultiSet,
        ordered_objects: false,
        set_keys: None,
    };
    let multiset_engine = DiffEngine::new(multiset_config);

    // Benchmark ordered array diff with identical arrays (best case for early termination)
    for size in [10, 100, 500, 1000].iter() {
        let json = generate_array(*size);
        let left: Value = sonic_rs::from_str(&json).unwrap();
        let right: Value = sonic_rs::from_str(&json).unwrap();

        group.bench_with_input(
            BenchmarkId::new("ordered_identical", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| ordered_engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

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
            BenchmarkId::new("ordered_10pct_diff", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| ordered_engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // Benchmark ordered array diff with high similarity (5% diff at end - tests suffix matching)
    for size in [10, 100, 500, 1000].iter() {
        let json1 = generate_array(*size);
        let json2 = {
            // Change only the last 5% of elements
            let change_start = (*size * 95) / 100;
            let elements: Vec<String> = (0..*size)
                .map(|i| if i >= change_start { (i + 1000).to_string() } else { i.to_string() })
                .collect();
            format!("[{}]", elements.join(", "))
        };

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("ordered_5pct_end_diff", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| ordered_engine.diff(black_box(*left), black_box(*right)).unwrap());
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
                b.iter(|| set_engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // Benchmark multiset mode with duplicates (few unique values)
    for size in [10, 100, 500, 1000].iter() {
        // Arrays with duplicates and different counts - only 10 unique values
        let elements1: Vec<String> = (0..*size).map(|i| (i % 10).to_string()).collect();
        let elements2: Vec<String> = (0..*size).map(|i| ((i + 1) % 10).to_string()).collect();

        let json1 = format!("[{}]", elements1.join(", "));
        let json2 = format!("[{}]", elements2.join(", "));

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("multiset_few_unique", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| multiset_engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // Benchmark multiset mode with unique values (true O(n²) stress test)
    for size in [10, 100, 500, 1000].iter() {
        // 50% overlap with all unique values - same pattern as set mode
        let elements1: Vec<String> = (0..*size).map(|i| i.to_string()).collect();
        let elements2: Vec<String> = (size / 2..size + size / 2).map(|i| i.to_string()).collect();

        let json1 = format!("[{}]", elements1.join(", "));
        let json2 = format!("[{}]", elements2.join(", "));

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("multiset_50pct_overlap", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| multiset_engine.diff(black_box(*left), black_box(*right)).unwrap());
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
        set_keys: None,
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
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    group.finish();
}

// ============================================================================
// Real-World Fixture Benchmarks
// ============================================================================

/// Load a fixture file and parse it as JSON
fn load_fixture(filename: &str) -> Option<Value> {
    let path = format!("{}/{}", FIXTURES_DIR, filename);
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Warning: Could not load fixture {}: {}", filename, e);
            return None;
        }
    };
    match sonic_rs::from_str(&content) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("Warning: Could not parse fixture {}: {}", filename, e);
            None
        }
    }
}

fn bench_real_world_fixtures(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world");

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
        set_keys: None,
    };
    let engine = DiffEngine::new(config);

    // India GeoJSON - three different sources of India's boundary with real structural differences
    if let (Some(india_osm), Some(india_composite), Some(india_soi)) = (
        load_fixture("india-osm.geojson"),
        load_fixture("india-composite.geojson"),
        load_fixture("india-soi.geojson"),
    ) {
        // Identical baseline (OSM vs OSM)
        group.bench_with_input(
            BenchmarkId::new("identical", "india_geojson"),
            &india_osm,
            |b, val| {
                b.iter(|| engine.diff(black_box(val), black_box(val)).unwrap());
            },
        );

        // Real diff: OSM (6.3MB) vs Composite (11MB)
        group.bench_with_input(
            BenchmarkId::new("diff", "india_osm_vs_composite"),
            &(&india_osm, &india_composite),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );

        // Real diff: OSM (6.3MB) vs SOI (12MB)
        group.bench_with_input(
            BenchmarkId::new("diff", "india_osm_vs_soi"),
            &(&india_osm, &india_soi),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );

        // Real diff: Composite (11MB) vs SOI (12MB)
        group.bench_with_input(
            BenchmarkId::new("diff", "india_composite_vs_soi"),
            &(&india_composite, &india_soi),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // VSCode package-lock.json comparisons - graduated diff percentages
    // Versions chosen to show ~8%, ~18%, ~42% differences
    // Note: Older VSCode versions used yarn.lock, not package-lock.json
    if let (Some(v1_94), Some(v1_95), Some(v1_100), Some(v1_108)) = (
        load_fixture("vscode-1.94-package-lock.json"),
        load_fixture("vscode-1.95-package-lock.json"),
        load_fixture("vscode-1.100-package-lock.json"),
        load_fixture("vscode-1.108-package-lock.json"),
    ) {
        // Identical baseline
        group.bench_with_input(
            BenchmarkId::new("identical", "vscode_package_lock"),
            &v1_95,
            |b, val| {
                b.iter(|| engine.diff(black_box(val), black_box(val)).unwrap());
            },
        );

        // ~8% diff: 1.95 vs 1.100 (small dependency update)
        group.bench_with_input(
            BenchmarkId::new("diff_8pct", "vscode_1.95_vs_1.100"),
            &(&v1_95, &v1_100),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );

        // ~18% diff: 1.94 vs 1.95 (minor version bump)
        group.bench_with_input(
            BenchmarkId::new("diff_18pct", "vscode_1.94_vs_1.95"),
            &(&v1_94, &v1_95),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );

        // ~42% diff: 1.94 vs 1.108 (major version gap)
        group.bench_with_input(
            BenchmarkId::new("diff_42pct", "vscode_1.94_vs_1.108"),
            &(&v1_94, &v1_108),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    group.finish();
}

// ============================================================================
// Set-Key Array Diff Benchmarks
// ============================================================================

/// Generate a JSON array of objects with id fields, wrapped in a root object.
/// Each object has: {"id": i, "name": "name_i", "value": "val_i"}
fn generate_keyed_array(n: usize, key: &str) -> String {
    let elements: Vec<String> = (0..n)
        .map(|i| format!(r#"{{"{}": {}, "name": "name_{}", "value": "val_{}"}}"#, key, i, i, i))
        .collect();
    format!(r#"{{"items": [{}]}}"#, elements.join(", "))
}

/// Generate a pair of keyed arrays where the second is reordered + has modifications.
/// `diff_percent` of elements have their "value" field changed.
/// `remove_count` elements are removed, `add_count` elements are added.
fn generate_keyed_diff_pair(
    n: usize,
    diff_percent: usize,
    remove_count: usize,
    add_count: usize,
) -> (String, String) {
    let diff_count = (n * diff_percent) / 100;

    // Left: sequential order
    let left_elements: Vec<String> = (0..n)
        .map(|i| format!(r#"{{"id": {}, "name": "name_{}", "value": "val_{}"}}"#, i, i, i))
        .collect();

    // Right: reverse order, with modifications, removals, and additions
    let mut right_elements: Vec<String> = (0..n)
        .rev()
        .filter(|i| *i >= remove_count) // remove first `remove_count` elements
        .map(|i| {
            if i < diff_count + remove_count {
                format!(
                    r#"{{"id": {}, "name": "name_{}", "value": "changed_{}"}}"#,
                    i, i, i
                )
            } else {
                format!(r#"{{"id": {}, "name": "name_{}", "value": "val_{}"}}"#, i, i, i)
            }
        })
        .collect();

    // Add new elements
    for i in 0..add_count {
        let new_id = n + i;
        right_elements.push(format!(
            r#"{{"id": {}, "name": "new_{}", "value": "new_val_{}"}}"#,
            new_id, new_id, new_id
        ));
    }

    (
        format!(r#"{{"items": [{}]}}"#, left_elements.join(", ")),
        format!(r#"{{"items": [{}]}}"#, right_elements.join(", ")),
    )
}

fn make_set_key_config(path: &str, key: &str) -> Option<SetKeyConfig> {
    let mut paths = HashMap::new();
    paths.insert(path.to_string(), vec![key.to_string()]);
    Some(SetKeyConfig {
        paths,
        allow_missing: true,
        allow_duplicates: true,
    })
}

fn bench_diff_set_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_set_key");

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
        set_keys: make_set_key_config("items", "id"),
    };
    let engine = DiffEngine::new(config);

    // Identical arrays (reordered) - best case: all matched, no diffs
    for size in [10, 100, 500, 1000].iter() {
        let json1 = generate_keyed_array(*size, "id");
        // Reverse the array order for the second file
        let elements_rev: Vec<String> = (0..*size)
            .rev()
            .map(|i| format!(r#"{{"id": {}, "name": "name_{}", "value": "val_{}"}}"#, i, i, i))
            .collect();
        let json2 = format!(r#"{{"items": [{}]}}"#, elements_rev.join(", "));

        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("reordered_identical", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // 10% of values modified, no additions/removals
    for size in [10, 100, 500, 1000].iter() {
        let (json1, json2) = generate_keyed_diff_pair(*size, 10, 0, 0);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("10pct_modified", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // 10% removed + 10% added (churn)
    for size in [10, 100, 500, 1000].iter() {
        let remove = *size / 10;
        let add = *size / 10;
        let (json1, json2) = generate_keyed_diff_pair(*size, 0, remove, add);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("10pct_churn", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
            },
        );
    }

    // 50% modified + 10% removed + 10% added (heavy diff)
    for size in [10, 100, 500, 1000].iter() {
        let remove = *size / 10;
        let add = *size / 10;
        let (json1, json2) = generate_keyed_diff_pair(*size, 50, remove, add);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();

        group.bench_with_input(
            BenchmarkId::new("50pct_modified_10pct_churn", size),
            &(&left, &right),
            |b, (left, right)| {
                b.iter(|| engine.diff(black_box(*left), black_box(*right)).unwrap());
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
    bench_real_world_fixtures,
    bench_diff_set_key,
);

criterion_main!(benches);
