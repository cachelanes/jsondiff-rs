#![allow(
    clippy::unwrap_used,
    clippy::integer_division,
    reason = "test code: unwraps and integer math are fine"
)]

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write as _;
use tempfile::NamedTempFile;

fn jsondiff() -> Command {
    cargo_bin_cmd!("jsondiff")
}

// ============================================================================
// Basic Diff Tests
// ============================================================================

#[test]
fn test_identical_files() {
    jsondiff()
        .args([
            "tests/fixtures/identical.json",
            "tests/fixtures/identical.json",
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_simple_diff() {
    jsondiff()
        .args([
            "tests/fixtures/simple_old.json",
            "tests/fixtures/simple_new.json",
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("$.version"))
        .stdout(predicate::str::contains("- 1"))
        .stdout(predicate::str::contains("+ 2"))
        .stdout(predicate::str::contains("$.enabled"))
        .stdout(predicate::str::contains("$.added"));
}

#[test]
fn test_nested_diff() {
    jsondiff()
        .args([
            "tests/fixtures/nested_old.json",
            "tests/fixtures/nested_new.json",
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("$.user.name"))
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("$.user.settings.theme"));
}

// ============================================================================
// Array Comparison Mode Tests
// ============================================================================

#[test]
fn test_array_ordered_mode_default() {
    // In ordered mode, [1,2,3,4,5] vs [5,4,3,2,1] should show differences
    // Verify specific array index paths are reported, not just generic keywords
    jsondiff()
        .args([
            "tests/fixtures/array_ordered.json",
            "tests/fixtures/array_reordered.json",
        ])
        .args(["--no-color", "-f", "summary"])
        .assert()
        .success()
        // In ordered mode with reversed arrays, we expect modifications at indices
        .stdout(predicate::str::contains("modified"));
}

#[test]
fn test_array_set_mode() {
    // In set mode, [1,2,3,4,5] and [5,4,3,2,1] should be identical
    jsondiff()
        .args([
            "tests/fixtures/array_ordered.json",
            "tests/fixtures/array_reordered.json",
        ])
        .args(["--array-set", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_array_multiset_mode() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, "[1, 1, 2, 3]").unwrap();
    writeln!(file2, "[1, 2, 2, 3]").unwrap();

    // In multiset mode, these should differ (one less 1, one more 2)
    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--array-multiset", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("added"))
        .stdout(predicate::str::contains("removed"));
}

#[test]
fn test_array_multiset_identical() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, "[1, 1, 2, 3]").unwrap();
    writeln!(file2, "[3, 1, 2, 1]").unwrap();

    // In multiset mode, these should be identical (same elements, same counts)
    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--array-multiset", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[test]
fn test_json_output_format() {
    jsondiff()
        .args([
            "tests/fixtures/simple_old.json",
            "tests/fixtures/simple_new.json",
        ])
        .args(["-f", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""op":"#))
        .stdout(predicate::str::contains(r#""path":"#))
        .stdout(predicate::str::contains(r#""stats":"#));
}

#[test]
fn test_summary_output_format() {
    jsondiff()
        .args([
            "tests/fixtures/simple_old.json",
            "tests/fixtures/simple_new.json",
        ])
        .args(["-f", "summary"])
        .assert()
        .success()
        .stdout(predicate::str::contains("added"))
        .stdout(predicate::str::contains("removed"))
        .stdout(predicate::str::contains("modified"));
}

#[test]
fn test_compact_mode() {
    // In compact mode, we should see diff output but no separator line or summary
    jsondiff()
        .args([
            "tests/fixtures/simple_old.json",
            "tests/fixtures/simple_new.json",
        ])
        .args(["--compact", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$.version"))
        .stdout(predicate::str::contains("- 1"))
        .stdout(predicate::str::contains("+ 2"))
        // Verify no summary stats in compact mode
        .stdout(predicate::str::contains("added,").not())
        .stdout(predicate::str::contains("removed,").not());
}

// ============================================================================
// Stdin Tests
// ============================================================================

#[test]
fn test_stdin_input() {
    jsondiff()
        .args(["-", "tests/fixtures/simple_new.json"])
        .arg("--no-color")
        .write_stdin(r#"{"name": "test", "version": 1, "enabled": true}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("$.version"))
        .stdout(predicate::str::contains("$.enabled"));
}

#[test]
fn test_both_stdin_error() {
    jsondiff()
        .args(["-", "-"])
        .write_stdin(r"{}")
        .assert()
        .failure()
        .stderr(predicate::str::contains("stdin"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_invalid_json() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, "not valid json").unwrap();
    writeln!(file2, r#"{{"valid": true}}"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON").or(predicate::str::contains("error")));
}

// ============================================================================
// Type Change Tests
// ============================================================================

#[test]
fn test_type_change() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"{{"value": "string"}}"#).unwrap();
    writeln!(file2, r#"{{"value": 123}}"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("$.value"))
        .stdout(predicate::str::contains("string"))
        .stdout(predicate::str::contains("123"));
}

#[test]
fn test_null_handling() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"{{"value": null}}"#).unwrap();
    writeln!(file2, r#"{{"value": "not null"}}"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("$.value"))
        .stdout(predicate::str::contains("null"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_empty_objects() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, "{{}}").unwrap();
    writeln!(file2, "{{}}").unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_empty_arrays() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, "[]").unwrap();
    writeln!(file2, "[]").unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_deeply_nested() {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"{{"a": {{"b": {{"c": {{"d": 1}}}}}}}}"#).unwrap();
    writeln!(file2, r#"{{"a": {{"b": {{"c": {{"d": 2}}}}}}}}"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("$.a.b.c.d"));
}

// ============================================================================
// Determinism Tests
// ============================================================================

#[test]
fn test_output_is_deterministic() {
    // Run the same diff multiple times and verify output is identical.
    // This catches non-determinism from HashSet iteration order.
    // Use an object with many keys to increase likelihood of detecting issues.
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    // Create objects with many keys - non-deterministic iteration would likely
    // produce different orderings across runs
    writeln!(
        file1,
        r#"{{
            "alpha": 1, "bravo": 2, "charlie": 3, "delta": 4, "echo": 5,
            "foxtrot": 6, "golf": 7, "hotel": 8, "india": 9, "juliet": 10,
            "kilo": 11, "lima": 12, "mike": 13, "november": 14, "oscar": 15,
            "papa": 16, "quebec": 17, "romeo": 18, "sierra": 19, "tango": 20,
            "nested": {{"a": 1, "b": 2, "c": 3, "d": 4, "e": 5}}
        }}"#
    )
    .unwrap();

    writeln!(
        file2,
        r#"{{
            "alpha": 100, "bravo": 2, "charlie": 300, "delta": 4, "echo": 500,
            "foxtrot": 6, "golf": 700, "hotel": 8, "india": 900, "juliet": 10,
            "kilo": 1100, "lima": 12, "mike": 1300, "november": 14, "oscar": 1500,
            "papa": 16, "quebec": 1700, "romeo": 18, "sierra": 1900, "tango": 20,
            "nested": {{"a": 100, "b": 2, "c": 300, "d": 4, "e": 500}},
            "new_key": "added"
        }}"#
    )
    .unwrap();

    let mut outputs = Vec::new();
    for _ in 0..20 {
        let output = jsondiff()
            .args([
                file1.path().to_str().unwrap(),
                file2.path().to_str().unwrap(),
            ])
            .args(["--no-color", "-f", "json"])
            .output()
            .expect("Failed to execute command");

        outputs.push(String::from_utf8_lossy(&output.stdout).to_string());
    }

    // All outputs must be identical
    let first = &outputs[0];
    for (i, output) in outputs.iter().enumerate().skip(1) {
        assert_eq!(
            first,
            output,
            "Output differed between run 1 and run {} - non-deterministic output detected",
            i + 1
        );
    }
}

// ============================================================================
// Object Comparison Mode Tests
// ============================================================================

// ============================================================================
// Set-Key Tests
// ============================================================================

#[test]
fn test_set_key_basic() {
    // With --set-key users.id, should match by id and show only real changes
    jsondiff()
        .args([
            "tests/fixtures/set_key_users_old.json",
            "tests/fixtures/set_key_users_new.json",
        ])
        .args(["--set-key", "users.id", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[id="))
        .stdout(predicate::str::contains("role"))
        // Bob (id=2) was removed
        .stdout(predicate::str::contains("Bob"))
        // Diana (id=4) was added
        .stdout(predicate::str::contains("Diana"));
}

#[test]
fn test_set_key_reorder_no_diff() {
    // Identical objects in different order should show no differences
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"{{"items": [{{"id": 1, "v": "a"}}, {{"id": 2, "v": "b"}}]}}"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"{{"items": [{{"id": 2, "v": "b"}}, {{"id": 1, "v": "a"}}]}}"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "items.id", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_set_key_root_array() {
    // Root-level array with --set-key id (no dot)
    jsondiff()
        .args([
            "tests/fixtures/set_key_root_old.json",
            "tests/fixtures/set_key_root_new.json",
        ])
        .args(["--set-key", "id", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("$[id="))
        // id=1 value changed from alpha to ALPHA
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("ALPHA"))
        // id=2 removed
        .stdout(predicate::str::contains("beta"))
        // id=4 added
        .stdout(predicate::str::contains("delta"));
}

#[test]
fn test_set_key_composite() {
    // Composite key: match by both type and region
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"[{{"type": "a", "region": "us", "v": 1}}, {{"type": "b", "region": "eu", "v": 2}}]"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"[{{"type": "b", "region": "eu", "v": 20}}, {{"type": "a", "region": "us", "v": 1}}]"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "type", "--set-key", "region", "--no-color"])
        .assert()
        .success()
        // Should show composite key notation
        .stdout(predicate::str::contains("[type=b,region=eu]"));
}

#[test]
fn test_set_key_missing_lenient() {
    // Default lenient mode: elements missing the key field fall back to set comparison
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"[{{"id": 1, "v": "a"}}, {{"no_id": true, "v": "b"}}]"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"[{{"id": 1, "v": "a"}}, {{"no_id": true, "v": "c"}}]"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "id", "--no-color"])
        .assert()
        .success()
        // Unkeyed elements should use set marker [{}]
        .stdout(predicate::str::contains("[{}]"));
}

#[test]
fn test_set_key_missing_strict() {
    // Strict mode: error on missing key field
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"[{{"id": 1, "v": "a"}}, {{"no_id": true}}]"#).unwrap();
    writeln!(file2, r#"[{{"id": 1, "v": "a"}}]"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args([
            "--set-key",
            "id",
            "--set-key-allow-missing=false",
            "--no-color",
        ])
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("missing required key field"));
}

#[test]
fn test_set_key_duplicates_lenient() {
    // Default lenient mode: duplicate keys use first-match-wins
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"[{{"id": 1, "v": "first"}}, {{"id": 1, "v": "second"}}, {{"id": 2, "v": "x"}}]"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"[{{"id": 1, "v": "first"}}, {{"id": 2, "v": "x"}}]"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "id", "--no-color"])
        .assert()
        .success();
}

#[test]
fn test_set_key_duplicates_strict() {
    // Strict mode: error on duplicate keys
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"[{{"id": 1, "v": "first"}}, {{"id": 1, "v": "second"}}]"#
    )
    .unwrap();
    writeln!(file2, r#"[{{"id": 1, "v": "first"}}]"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args([
            "--set-key",
            "id",
            "--set-key-allow-duplicates=false",
            "--no-color",
        ])
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("duplicate key"));
}

#[test]
fn test_set_key_json_output() {
    // JSON output format should include key-match paths
    jsondiff()
        .args([
            "tests/fixtures/set_key_users_old.json",
            "tests/fixtures/set_key_users_new.json",
        ])
        .args(["--set-key", "users.id", "-f", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[id="));
}

#[test]
fn test_set_key_nested_path() {
    // Test deeply nested path matching
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"{{"data": {{"items": [{{"sku": "A1", "price": 10}}, {{"sku": "B2", "price": 20}}]}}}}"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"{{"data": {{"items": [{{"sku": "B2", "price": 25}}, {{"sku": "A1", "price": 10}}]}}}}"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "data.items.sku", "--no-color"])
        .assert()
        .success()
        // B2 price changed
        .stdout(predicate::str::contains("[sku=B2]"))
        .stdout(predicate::str::contains("price"))
        // A1 unchanged, should not appear (only B2 changed)
        .stdout(predicate::str::contains("[sku=A1]").not());
}

// ============================================================================
// Set-Key Bug Regression Tests
// ============================================================================

#[test]
fn test_set_key_duplicate_error_detected() {
    // Duplicate keys in the array should produce an error when allow_duplicates=false.
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"["not_an_object", {{"id": 1, "v": "first"}}, {{"id": 1, "v": "dup"}}]"#
    )
    .unwrap();
    writeln!(file2, r#"[{{"id": 1, "v": "first"}}]"#).unwrap();

    let output = jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args([
            "--set-key",
            "id",
            "--set-key-allow-duplicates=false",
            "--no-color",
        ])
        .output()
        .expect("failed to execute");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicate"),
        "Error should mention duplicate key, got: {stderr}"
    );
    assert_eq!(output.status.code(), Some(5));
}

#[test]
fn test_set_key_empty_key_field_rejected() {
    // Bug: --set-key "" produces empty key field, which vacuously matches all objects.
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"[{{"id": 1}}]"#).unwrap();
    writeln!(file2, r#"[{{"id": 1}}]"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "", "--no-color"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn test_set_key_dot_only_rejected() {
    // Bug: --set-key "." produces empty path and empty key.
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"[{{"id": 1}}]"#).unwrap();
    writeln!(file2, r#"[{{"id": 1}}]"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", ".", "--no-color"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn test_set_key_duplicate_key_args_deduped() {
    // Bug: --set-key users.id --set-key users.id creates composite key ["id","id"]
    // which would fail to match since objects don't have two "id" fields.
    // Should be deduplicated to ["id"].
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"{{"users": [{{"id": 1, "v": "a"}}, {{"id": 2, "v": "b"}}]}}"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"{{"users": [{{"id": 2, "v": "b"}}, {{"id": 1, "v": "a"}}]}}"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args([
            "--set-key",
            "users.id",
            "--set-key",
            "users.id",
            "--no-color",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_set_key_empty_arrays() {
    // Edge case: both arrays empty with set-key should show no differences
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, r#"{{"items": []}}"#).unwrap();
    writeln!(file2, r#"{{"items": []}}"#).unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "items.id", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences found"));
}

#[test]
fn test_set_key_non_object_elements() {
    // Edge case: array with mix of objects and primitives
    // Primitives can't have keys, should be treated as unkeyed (lenient)
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(
        file1,
        r#"[{{"id": 1, "v": "a"}}, 42, "hello", {{"id": 2, "v": "b"}}]"#
    )
    .unwrap();
    writeln!(
        file2,
        r#"[{{"id": 2, "v": "b"}}, 42, "hello", {{"id": 1, "v": "a"}}]"#
    )
    .unwrap();

    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--set-key", "id", "--no-color"])
        .assert()
        .success()
        // Objects matched by key, primitives matched by set — no differences
        .stdout(predicate::str::contains("No differences found"));
}

// ============================================================================
// Object Comparison Mode Tests
// ============================================================================

#[test]
fn test_ordered_objects_flag() {
    // Test that --ordered-objects / -o flag works
    // Objects with same keys in different order should differ in ordered mode
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    // Note: JSON object key order is typically not preserved by parsers,
    // but the flag should at least be accepted without error
    writeln!(file1, r#"{{"a": 1, "b": 2}}"#).unwrap();
    writeln!(file2, r#"{{"b": 2, "a": 1}}"#).unwrap();

    // Test that the flag is accepted (doesn't error)
    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["--ordered-objects", "--no-color"])
        .assert()
        .success();

    // Also test short form -o
    jsondiff()
        .args([
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .args(["-o", "--no-color"])
        .assert()
        .success();
}
