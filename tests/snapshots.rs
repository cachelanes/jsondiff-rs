use assert_cmd::Command;
use insta::assert_snapshot;

fn jsondiff_output(args: &[&str]) -> String {
    let output = Command::cargo_bin("jsondiff")
        .unwrap()
        .args(args)
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).to_string()
}

// ============================================================================
// Pretty Output Snapshots
// ============================================================================

#[test]
fn snapshot_simple_diff_pretty() {
    let output = jsondiff_output(&[
        "tests/fixtures/simple_old.json",
        "tests/fixtures/simple_new.json",
        "--no-color",
    ]);
    assert_snapshot!("simple_diff_pretty", output);
}

#[test]
fn snapshot_nested_diff_pretty() {
    let output = jsondiff_output(&[
        "tests/fixtures/nested_old.json",
        "tests/fixtures/nested_new.json",
        "--no-color",
    ]);
    assert_snapshot!("nested_diff_pretty", output);
}

#[test]
fn snapshot_identical_files() {
    let output = jsondiff_output(&[
        "tests/fixtures/identical.json",
        "tests/fixtures/identical.json",
        "--no-color",
    ]);
    assert_snapshot!("identical_files", output);
}

#[test]
fn snapshot_array_ordered_diff() {
    let output = jsondiff_output(&[
        "tests/fixtures/array_ordered.json",
        "tests/fixtures/array_reordered.json",
        "--no-color",
    ]);
    assert_snapshot!("array_ordered_diff", output);
}

#[test]
fn snapshot_array_set_mode() {
    let output = jsondiff_output(&[
        "tests/fixtures/array_ordered.json",
        "tests/fixtures/array_reordered.json",
        "--array-set",
        "--no-color",
    ]);
    assert_snapshot!("array_set_mode", output);
}

// ============================================================================
// JSON Output Snapshots
// ============================================================================

#[test]
fn snapshot_simple_diff_json() {
    let output = jsondiff_output(&[
        "tests/fixtures/simple_old.json",
        "tests/fixtures/simple_new.json",
        "-f",
        "json",
    ]);
    assert_snapshot!("simple_diff_json", output);
}

#[test]
fn snapshot_nested_diff_json() {
    let output = jsondiff_output(&[
        "tests/fixtures/nested_old.json",
        "tests/fixtures/nested_new.json",
        "-f",
        "json",
    ]);
    assert_snapshot!("nested_diff_json", output);
}

// ============================================================================
// Summary Output Snapshots
// ============================================================================

#[test]
fn snapshot_simple_diff_summary() {
    let output = jsondiff_output(&[
        "tests/fixtures/simple_old.json",
        "tests/fixtures/simple_new.json",
        "-f",
        "summary",
    ]);
    assert_snapshot!("simple_diff_summary", output);
}

#[test]
fn snapshot_nested_diff_summary() {
    let output = jsondiff_output(&[
        "tests/fixtures/nested_old.json",
        "tests/fixtures/nested_new.json",
        "-f",
        "summary",
    ]);
    assert_snapshot!("nested_diff_summary", output);
}

// ============================================================================
// Compact Mode Snapshots
// ============================================================================

#[test]
fn snapshot_simple_diff_compact() {
    let output = jsondiff_output(&[
        "tests/fixtures/simple_old.json",
        "tests/fixtures/simple_new.json",
        "--compact",
        "--no-color",
    ]);
    assert_snapshot!("simple_diff_compact", output);
}
