use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

fn jsondiff() -> Command {
    Command::cargo_bin("jsondiff").unwrap()
}

// ============================================================================
// Basic CLI Tests
// ============================================================================

#[test]
fn test_help_flag() {
    jsondiff()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON comparison tool"))
        .stdout(predicate::str::contains("--array-set"))
        .stdout(predicate::str::contains("--array-multiset"));
}

#[test]
fn test_version_flag() {
    jsondiff()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("jsondiff"));
}

#[test]
fn test_missing_arguments() {
    jsondiff()
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_file_not_found() {
    jsondiff()
        .args(["nonexistent.json", "also_nonexistent.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("File not found")));
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
    jsondiff()
        .args([
            "tests/fixtures/array_ordered.json",
            "tests/fixtures/array_reordered.json",
        ])
        .arg("--no-color")
        .assert()
        .success()
        .stdout(predicate::str::contains("added").or(predicate::str::contains("removed")).or(predicate::str::contains("modified")));
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
    // In compact mode, we should not see the separator line or summary
    jsondiff()
        .args([
            "tests/fixtures/identical.json",
            "tests/fixtures/identical.json",
        ])
        .args(["--compact", "--no-color"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
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
        .write_stdin(r#"{}"#)
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
