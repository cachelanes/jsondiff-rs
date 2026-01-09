//! Benchmark comparing 5 deterministic output approaches for diff_objects
//!
//! Run with: cargo bench --bench deterministic_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use indexmap::IndexSet;
use sonic_rs::{JsonContainerTrait, Object, Value};
use std::collections::{BTreeSet, HashSet};

use jsondiff::diff::engine::diff_values;
use jsondiff::diff::object::values_equal;
use jsondiff::diff::types::{ArrayCompareMode, DiffConfig, DiffOp, JsonPath};

// ============================================================================
// Implementation Variants
// ============================================================================

/// Approach 1: Sort keys before iterating (collect HashSet to Vec, sort, iterate)
fn diff_objects_sorted_keys(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k).collect();
    let right_keys: HashSet<&str> = right.iter().map(|(k, _)| k).collect();

    // Keys only in left (removed) - sorted for determinism
    let mut removed_keys: Vec<&str> = left_keys.difference(&right_keys).copied().collect();
    removed_keys.sort_unstable();
    for key in removed_keys {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Removed {
            path: child_path,
            value: left.get(&key.to_string()).unwrap().clone(),
        });
    }

    // Keys only in right (added) - sorted for determinism
    let mut added_keys: Vec<&str> = right_keys.difference(&left_keys).copied().collect();
    added_keys.sort_unstable();
    for key in added_keys {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Added {
            path: child_path,
            value: right.get(&key.to_string()).unwrap().clone(),
        });
    }

    // Keys in both (common) - sorted for determinism
    if config.ordered_objects {
        let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
        let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

        let mut common_keys: Vec<&str> = left_keys.intersection(&right_keys).copied().collect();
        common_keys.sort_unstable();

        for key in common_keys {
            let left_pos = left_order.iter().position(|k| k == &key);
            let right_pos = right_order.iter().position(|k| k == &key);

            if left_pos != right_pos {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();

                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        }
    } else {
        let mut common_keys: Vec<&str> = left_keys.intersection(&right_keys).copied().collect();
        common_keys.sort_unstable();

        for key in common_keys {
            let child_path = path.append_key(key);
            let left_val = left.get(&key.to_string()).unwrap();
            let right_val = right.get(&key.to_string()).unwrap();
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        }
    }

    ops
}

/// Approach 2: Use BTreeSet for inherent ordering
fn diff_objects_btreeset(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let left_keys: BTreeSet<&str> = left.iter().map(|(k, _)| k).collect();
    let right_keys: BTreeSet<&str> = right.iter().map(|(k, _)| k).collect();

    // Keys only in left (removed) - already sorted
    for key in left_keys.difference(&right_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Removed {
            path: child_path,
            value: left.get(&key.to_string()).unwrap().clone(),
        });
    }

    // Keys only in right (added) - already sorted
    for key in right_keys.difference(&left_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Added {
            path: child_path,
            value: right.get(&key.to_string()).unwrap().clone(),
        });
    }

    // Keys in both (common) - already sorted
    if config.ordered_objects {
        let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
        let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

        for key in left_keys.intersection(&right_keys) {
            let left_pos = left_order.iter().position(|k| k == key);
            let right_pos = right_order.iter().position(|k| k == key);

            if left_pos != right_pos {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();

                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        }
    } else {
        for key in left_keys.intersection(&right_keys) {
            let child_path = path.append_key(key);
            let left_val = left.get(&key.to_string()).unwrap();
            let right_val = right.get(&key.to_string()).unwrap();
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        }
    }

    ops
}

/// Approach 3: Preserve source JSON key order (iterate left.iter() directly)
fn diff_objects_preserve_order(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let right_keys: HashSet<&str> = right.iter().map(|(k, _)| k).collect();

    if config.ordered_objects {
        let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
        let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

        for (key, left_val) in left.iter() {
            let child_path = path.append_key(key);

            if !right_keys.contains(key) {
                ops.push(DiffOp::Removed {
                    path: child_path,
                    value: left_val.clone(),
                });
            } else {
                let left_pos = left_order.iter().position(|k| *k == key);
                let right_pos = right_order.iter().position(|k| *k == key);

                if left_pos != right_pos {
                    let right_val = right.get(&key.to_string()).unwrap();
                    if !values_equal(left_val, right_val) {
                        ops.push(DiffOp::Modified {
                            path: child_path,
                            old_value: left_val.clone(),
                            new_value: right_val.clone(),
                        });
                    }
                } else {
                    let right_val = right.get(&key.to_string()).unwrap();
                    ops.extend(diff_values(left_val, right_val, &child_path, config));
                }
            }
        }
    } else {
        for (key, left_val) in left.iter() {
            let child_path = path.append_key(key);

            if !right_keys.contains(key) {
                ops.push(DiffOp::Removed {
                    path: child_path,
                    value: left_val.clone(),
                });
            } else {
                let right_val = right.get(&key.to_string()).unwrap();
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        }
    }

    // Keys only in right (in right's source order)
    let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k).collect();
    for (key, right_val) in right.iter() {
        if !left_keys.contains(key) {
            let child_path = path.append_key(key);
            ops.push(DiffOp::Added {
                path: child_path,
                value: right_val.clone(),
            });
        }
    }

    ops
}

/// Approach 4: Sort DiffOps by path at the end
fn diff_objects_sort_output(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let left_keys: HashSet<&str> = left.iter().map(|(k, _)| k).collect();
    let right_keys: HashSet<&str> = right.iter().map(|(k, _)| k).collect();

    for key in left_keys.difference(&right_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Removed {
            path: child_path,
            value: left.get(&key.to_string()).unwrap().clone(),
        });
    }

    for key in right_keys.difference(&left_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Added {
            path: child_path,
            value: right.get(&key.to_string()).unwrap().clone(),
        });
    }

    if config.ordered_objects {
        let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
        let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();
        let common_keys: HashSet<&str> = left_keys.intersection(&right_keys).copied().collect();

        for key in &common_keys {
            let left_pos = left_order.iter().position(|k| k == key);
            let right_pos = right_order.iter().position(|k| k == key);

            if left_pos != right_pos {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();

                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        }
    } else {
        for key in left_keys.intersection(&right_keys) {
            let child_path = path.append_key(key);
            let left_val = left.get(&key.to_string()).unwrap();
            let right_val = right.get(&key.to_string()).unwrap();
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        }
    }

    // Sort operations by path for deterministic output
    ops.sort_by(|a, b| {
        let path_a = match a {
            DiffOp::Added { path, .. } => path,
            DiffOp::Removed { path, .. } => path,
            DiffOp::Modified { path, .. } => path,
        };
        let path_b = match b {
            DiffOp::Added { path, .. } => path,
            DiffOp::Removed { path, .. } => path,
            DiffOp::Modified { path, .. } => path,
        };
        path_a.to_string().cmp(&path_b.to_string())
    });

    ops
}

/// Approach 5: Use IndexSet for insertion-order preserving
fn diff_objects_indexset(
    left: &Object,
    right: &Object,
    path: &JsonPath,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let left_keys: IndexSet<&str> = left.iter().map(|(k, _)| k).collect();
    let right_keys: IndexSet<&str> = right.iter().map(|(k, _)| k).collect();

    for key in left_keys.difference(&right_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Removed {
            path: child_path,
            value: left.get(&key.to_string()).unwrap().clone(),
        });
    }

    for key in right_keys.difference(&left_keys) {
        let child_path = path.append_key(key);
        ops.push(DiffOp::Added {
            path: child_path,
            value: right.get(&key.to_string()).unwrap().clone(),
        });
    }

    if config.ordered_objects {
        let left_order: Vec<&str> = left.iter().map(|(k, _)| k).collect();
        let right_order: Vec<&str> = right.iter().map(|(k, _)| k).collect();

        for key in left_keys.intersection(&right_keys) {
            let left_pos = left_order.iter().position(|k| k == key);
            let right_pos = right_order.iter().position(|k| k == key);

            if left_pos != right_pos {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();

                if !values_equal(left_val, right_val) {
                    ops.push(DiffOp::Modified {
                        path: child_path,
                        old_value: left_val.clone(),
                        new_value: right_val.clone(),
                    });
                }
            } else {
                let child_path = path.append_key(key);
                let left_val = left.get(&key.to_string()).unwrap();
                let right_val = right.get(&key.to_string()).unwrap();
                ops.extend(diff_values(left_val, right_val, &child_path, config));
            }
        }
    } else {
        for key in left_keys.intersection(&right_keys) {
            let child_path = path.append_key(key);
            let left_val = left.get(&key.to_string()).unwrap();
            let right_val = right.get(&key.to_string()).unwrap();
            ops.extend(diff_values(left_val, right_val, &child_path, config));
        }
    }

    ops
}

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

/// Generate two objects with specified percentage of value differences
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

    (
        format!("{{{}}}", parts1.join(", ")),
        format!("{{{}}}", parts2.join(", ")),
    )
}

/// Generate two completely different objects (all keys differ)
fn generate_completely_different(n: usize) -> (String, String) {
    let mut parts1 = Vec::with_capacity(n);
    let mut parts2 = Vec::with_capacity(n);

    for i in 0..n {
        parts1.push(format!(r#""old_key_{}": "old_value_{}""#, i, i));
        parts2.push(format!(r#""new_key_{}": "new_value_{}""#, i, i));
    }

    (
        format!("{{{}}}", parts1.join(", ")),
        format!("{{{}}}", parts2.join(", ")),
    )
}

/// Generate a nested JSON structure with specified depth and breadth
fn generate_nested(depth: usize, breadth: usize) -> String {
    if depth == 0 {
        return r#""leaf""#.to_string();
    }

    let mut parts = Vec::with_capacity(breadth);
    for i in 0..breadth {
        parts.push(format!(
            r#""child_{}": {}"#,
            i,
            generate_nested(depth - 1, breadth)
        ));
    }
    format!("{{{}}}", parts.join(", "))
}

/// Generate nested pair with differences at leaf level
fn generate_nested_diff_pair(depth: usize, breadth: usize) -> (String, String) {
    let json1 = generate_nested(depth, breadth);
    let json2 = json1.replace("leaf", "changed");
    (json1, json2)
}

// ============================================================================
// Type alias for diff function signatures
// ============================================================================

type DiffFn = fn(&Object, &Object, &JsonPath, &DiffConfig) -> Vec<DiffOp>;

// ============================================================================
// Benchmark Groups
// ============================================================================

fn bench_deterministic_approaches(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic_diff_objects");

    // Configure for statistical significance
    group.sample_size(100);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(5));

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let path = JsonPath::root();

    // Define all approaches
    let approaches: Vec<(&str, DiffFn)> = vec![
        ("1_sorted_keys", diff_objects_sorted_keys),
        ("2_btreeset", diff_objects_btreeset),
        ("3_preserve_order", diff_objects_preserve_order),
        ("4_sort_output", diff_objects_sort_output),
        ("5_indexset", diff_objects_indexset),
    ];

    // Test sizes: 10, 100, 1000, 5000 keys
    let sizes = [10, 100, 1000, 5000];

    // Test scenarios: identical (0%), 10% diff, 50% diff, 100% diff
    let scenarios: Vec<(&str, fn(usize) -> (String, String))> = vec![
        ("identical", |n| {
            let json = generate_object(n);
            (json.clone(), json)
        }),
        ("10pct_diff", |n| generate_diff_pair(n, 10)),
        ("50pct_diff", |n| generate_diff_pair(n, 50)),
        ("100pct_diff", |n| generate_completely_different(n)),
    ];

    for size in sizes.iter() {
        for (scenario_name, scenario_fn) in &scenarios {
            let (json1, json2) = scenario_fn(*size);
            let left: Value = sonic_rs::from_str(&json1).unwrap();
            let right: Value = sonic_rs::from_str(&json2).unwrap();
            let left_obj = left.as_object().unwrap();
            let right_obj = right.as_object().unwrap();

            // Set throughput for comparison
            group.throughput(Throughput::Elements(*size as u64));

            for (approach_name, approach_fn) in &approaches {
                group.bench_with_input(
                    BenchmarkId::new(format!("{}/{}", scenario_name, approach_name), size),
                    &(left_obj, right_obj, &path, &config),
                    |b, (left_obj, right_obj, path, config)| {
                        b.iter(|| {
                            approach_fn(
                                black_box(*left_obj),
                                black_box(*right_obj),
                                black_box(*path),
                                black_box(*config),
                            )
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// Benchmark comparing ordered_objects=true vs false for each approach
fn bench_ordered_objects_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("ordered_objects_mode");

    group.sample_size(100);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(5));

    let path = JsonPath::root();

    let approaches: Vec<(&str, DiffFn)> = vec![
        ("1_sorted_keys", diff_objects_sorted_keys),
        ("2_btreeset", diff_objects_btreeset),
        ("3_preserve_order", diff_objects_preserve_order),
        ("4_sort_output", diff_objects_sort_output),
        ("5_indexset", diff_objects_indexset),
    ];

    // Test with 1000 keys, 50% diff scenario
    let (json1, json2) = generate_diff_pair(1000, 50);
    let left: Value = sonic_rs::from_str(&json1).unwrap();
    let right: Value = sonic_rs::from_str(&json2).unwrap();
    let left_obj = left.as_object().unwrap();
    let right_obj = right.as_object().unwrap();

    for ordered in [false, true] {
        let config = DiffConfig {
            array_mode: ArrayCompareMode::Ordered,
            ordered_objects: ordered,
        };
        let mode_name = if ordered { "ordered" } else { "unordered" };

        for (approach_name, approach_fn) in &approaches {
            group.bench_with_input(
                BenchmarkId::new(format!("{}/{}", mode_name, approach_name), 1000),
                &(left_obj, right_obj, &path, &config),
                |b, (left_obj, right_obj, path, config)| {
                    b.iter(|| {
                        approach_fn(
                            black_box(*left_obj),
                            black_box(*right_obj),
                            black_box(*path),
                            black_box(*config),
                        )
                    });
                },
            );
        }
    }

    group.finish();
}

/// Scaling characteristics benchmark
fn bench_scaling_characteristics(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_characteristics");

    group.sample_size(50);
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(3));

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let path = JsonPath::root();

    let approaches: Vec<(&str, DiffFn)> = vec![
        ("1_sorted_keys", diff_objects_sorted_keys),
        ("2_btreeset", diff_objects_btreeset),
        ("3_preserve_order", diff_objects_preserve_order),
        ("4_sort_output", diff_objects_sort_output),
        ("5_indexset", diff_objects_indexset),
    ];

    // Extended sizes to see scaling behavior
    let sizes = [100, 500, 1000, 2500, 5000, 7500, 10000];

    for size in sizes.iter() {
        let (json1, json2) = generate_diff_pair(*size, 50);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();
        let left_obj = left.as_object().unwrap();
        let right_obj = right.as_object().unwrap();

        group.throughput(Throughput::Elements(*size as u64));

        for (approach_name, approach_fn) in &approaches {
            group.bench_with_input(
                BenchmarkId::new(*approach_name, size),
                &(left_obj, right_obj, &path, &config),
                |b, (left_obj, right_obj, path, config)| {
                    b.iter(|| {
                        approach_fn(
                            black_box(*left_obj),
                            black_box(*right_obj),
                            black_box(*path),
                            black_box(*config),
                        )
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark nested structures (tests recursive diff behavior)
fn bench_nested_deterministic(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_deterministic");

    group.sample_size(100);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(5));

    let config = DiffConfig {
        array_mode: ArrayCompareMode::Ordered,
        ordered_objects: false,
    };
    let path = JsonPath::root();

    let approaches: Vec<(&str, DiffFn)> = vec![
        ("1_sorted_keys", diff_objects_sorted_keys),
        ("2_btreeset", diff_objects_btreeset),
        ("3_preserve_order", diff_objects_preserve_order),
        ("4_sort_output", diff_objects_sort_output),
        ("5_indexset", diff_objects_indexset),
    ];

    // Test nested structures with varying depth and breadth (like original benchmarks)
    // Depths: 2, 4, 6 with breadth 3 (same as existing benchmarks)
    for depth in [2, 4, 6] {
        let breadth = 3;
        let (json1, json2) = generate_nested_diff_pair(depth, breadth);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();
        let left_obj = left.as_object().unwrap();
        let right_obj = right.as_object().unwrap();

        // Calculate total nodes for throughput: breadth^0 + breadth^1 + ... + breadth^depth
        let total_nodes: u64 = (0..=depth).map(|d| (breadth as u64).pow(d as u32)).sum();
        group.throughput(Throughput::Elements(total_nodes));

        for (approach_name, approach_fn) in &approaches {
            group.bench_with_input(
                BenchmarkId::new(format!("depth_{}/{}", depth, approach_name), breadth),
                &(left_obj, right_obj, &path, &config),
                |b, (left_obj, right_obj, path, config)| {
                    b.iter(|| {
                        approach_fn(
                            black_box(*left_obj),
                            black_box(*right_obj),
                            black_box(*path),
                            black_box(*config),
                        )
                    });
                },
            );
        }
    }

    // Also test with higher breadth (5) at depth 3
    for breadth in [5, 10] {
        let depth = 3;
        let (json1, json2) = generate_nested_diff_pair(depth, breadth);
        let left: Value = sonic_rs::from_str(&json1).unwrap();
        let right: Value = sonic_rs::from_str(&json2).unwrap();
        let left_obj = left.as_object().unwrap();
        let right_obj = right.as_object().unwrap();

        let total_nodes: u64 = (0..=depth).map(|d| (breadth as u64).pow(d as u32)).sum();
        group.throughput(Throughput::Elements(total_nodes));

        for (approach_name, approach_fn) in &approaches {
            group.bench_with_input(
                BenchmarkId::new(format!("breadth_{}/{}", breadth, approach_name), depth),
                &(left_obj, right_obj, &path, &config),
                |b, (left_obj, right_obj, path, config)| {
                    b.iter(|| {
                        approach_fn(
                            black_box(*left_obj),
                            black_box(*right_obj),
                            black_box(*path),
                            black_box(*config),
                        )
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_deterministic_approaches,
    bench_ordered_objects_mode,
    bench_scaling_characteristics,
    bench_nested_deterministic,
);

criterion_main!(benches);
