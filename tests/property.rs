use proptest::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

// ============================================================================
// Property Test Strategies
// ============================================================================

/// Generate arbitrary JSON-like values as strings
fn arb_json_primitive() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("null".to_string()),
        any::<bool>().prop_map(|b| b.to_string()),
        any::<i32>().prop_map(|n| n.to_string()),
        any::<f64>()
            .prop_filter("finite float", |f| f.is_finite())
            .prop_map(|f| f.to_string()),
        "[a-zA-Z0-9_ ]{0,20}".prop_map(|s| format!("\"{}\"", s)),
    ]
}

/// Generate simple JSON objects
fn arb_simple_json_object() -> impl Strategy<Value = String> {
    prop::collection::vec(
        (
            "[a-z]{1,10}",
            prop_oneof![
                Just("null".to_string()),
                any::<bool>().prop_map(|b| b.to_string()),
                any::<i32>().prop_map(|n| n.to_string()),
                "[a-zA-Z0-9]{0,10}".prop_map(|s| format!("\"{}\"", s)),
            ],
        ),
        0..5,
    )
    .prop_map(|pairs| {
        let fields: Vec<String> = pairs
            .into_iter()
            .map(|(k, v)| format!("\"{}\": {}", k, v))
            .collect();
        format!("{{{}}}", fields.join(", "))
    })
}

/// Generate simple JSON arrays
fn arb_simple_json_array() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_json_primitive(), 0..10).prop_map(|items| {
        format!("[{}]", items.join(", "))
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn run_jsondiff(json1: &str, json2: &str, extra_args: &[&str]) -> std::process::Output {
    let mut file1 = NamedTempFile::new().unwrap();
    let mut file2 = NamedTempFile::new().unwrap();

    writeln!(file1, "{}", json1).unwrap();
    writeln!(file2, "{}", json2).unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_jsondiff"));
    cmd.arg(file1.path())
        .arg(file2.path())
        .arg("--no-color")
        .args(extra_args);

    cmd.output().expect("Failed to execute jsondiff")
}

fn jsondiff_succeeds(json1: &str, json2: &str, extra_args: &[&str]) -> bool {
    run_jsondiff(json1, json2, extra_args).status.success()
}

// ============================================================================
// Property Tests: Reflexivity
// ============================================================================

proptest! {
    /// Diffing a value against itself should always produce no differences
    #[test]
    fn prop_diff_self_is_empty(json in arb_simple_json_object()) {
        let output = run_jsondiff(&json, &json, &[]);
        prop_assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        prop_assert!(stdout.contains("No differences found"), "Expected no differences for identical JSON: {}", json);
    }

    /// Diffing an array against itself should always produce no differences
    #[test]
    fn prop_array_diff_self_is_empty(json in arb_simple_json_array()) {
        let output = run_jsondiff(&json, &json, &[]);
        prop_assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        prop_assert!(stdout.contains("No differences found"), "Expected no differences for identical array: {}", json);
    }
}

// ============================================================================
// Property Tests: Array Set Mode
// ============================================================================

proptest! {
    /// In set mode, arrays with same elements in different order should be equal
    #[test]
    fn prop_array_set_order_independent(
        elements in prop::collection::vec(any::<i32>(), 1..10)
    ) {
        let arr1: Vec<String> = elements.iter().map(|n| n.to_string()).collect();
        let mut arr2 = arr1.clone();
        arr2.reverse();

        let json1 = format!("[{}]", arr1.join(", "));
        let json2 = format!("[{}]", arr2.join(", "));

        let output = run_jsondiff(&json1, &json2, &["--array-set"]);
        prop_assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        prop_assert!(stdout.contains("No differences found"),
            "Expected no differences in set mode for {} vs {}", json1, json2);
    }

    /// In multiset mode, arrays with same element counts should be equal
    #[test]
    fn prop_array_multiset_count_based(
        elements in prop::collection::vec(1..5i32, 1..8)
    ) {
        let arr1: Vec<String> = elements.iter().map(|n| n.to_string()).collect();
        let mut arr2 = arr1.clone();
        arr2.sort();
        arr2.reverse();

        let json1 = format!("[{}]", arr1.join(", "));
        let json2 = format!("[{}]", arr2.join(", "));

        let output = run_jsondiff(&json1, &json2, &["--array-multiset"]);
        prop_assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        prop_assert!(stdout.contains("No differences found"),
            "Expected no differences in multiset mode for {} vs {}", json1, json2);
    }
}

// ============================================================================
// Property Tests: Output Format Consistency
// ============================================================================

proptest! {
    /// JSON output format should always be valid JSON
    #[test]
    fn prop_json_output_is_valid(
        json1 in arb_simple_json_object(),
        json2 in arb_simple_json_object()
    ) {
        let output = run_jsondiff(&json1, &json2, &["-f", "json"]);
        prop_assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Basic check that it looks like JSON
        prop_assert!(stdout.trim().starts_with('{') && stdout.trim().ends_with('}'),
            "JSON output should be a valid JSON object: {}", stdout);
    }

    /// Summary output should contain the expected keywords
    #[test]
    fn prop_summary_output_format(
        json1 in arb_simple_json_object(),
        json2 in arb_simple_json_object()
    ) {
        let output = run_jsondiff(&json1, &json2, &["-f", "summary"]);
        prop_assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        prop_assert!(stdout.contains("added"), "Summary should mention 'added': {}", stdout);
        prop_assert!(stdout.contains("removed"), "Summary should mention 'removed': {}", stdout);
        prop_assert!(stdout.contains("modified"), "Summary should mention 'modified': {}", stdout);
    }
}

// ============================================================================
// Property Tests: No Crashes
// ============================================================================

proptest! {
    /// The tool should never crash on valid JSON input
    #[test]
    fn prop_no_crash_on_valid_json(
        json1 in arb_simple_json_object(),
        json2 in arb_simple_json_object()
    ) {
        prop_assert!(jsondiff_succeeds(&json1, &json2, &[]));
    }

    /// The tool should never crash on array input
    #[test]
    fn prop_no_crash_on_arrays(
        json1 in arb_simple_json_array(),
        json2 in arb_simple_json_array()
    ) {
        prop_assert!(jsondiff_succeeds(&json1, &json2, &[]));
    }

    /// The tool should handle all array modes without crashing
    #[test]
    fn prop_no_crash_array_modes(
        json1 in arb_simple_json_array(),
        json2 in arb_simple_json_array(),
        mode in prop_oneof![Just(""), Just("--array-set"), Just("--array-multiset")]
    ) {
        let args: Vec<&str> = if mode.is_empty() { vec![] } else { vec![mode] };
        prop_assert!(jsondiff_succeeds(&json1, &json2, &args));
    }
}

// ============================================================================
// Property Tests: Commutativity in Set Mode
// ============================================================================

proptest! {
    /// In set mode, diff(A, B) and diff(B, A) should have symmetric counts
    #[test]
    fn prop_set_mode_symmetric_counts(
        elements1 in prop::collection::vec(1..10i32, 0..5),
        elements2 in prop::collection::vec(1..10i32, 0..5)
    ) {
        let json1 = format!("[{}]", elements1.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", "));
        let json2 = format!("[{}]", elements2.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", "));

        let output1 = run_jsondiff(&json1, &json2, &["--array-set", "-f", "summary"]);
        let output2 = run_jsondiff(&json2, &json1, &["--array-set", "-f", "summary"]);

        prop_assert!(output1.status.success());
        prop_assert!(output2.status.success());

        // Parse the summaries to check symmetric counts
        let stdout1 = String::from_utf8_lossy(&output1.stdout);
        let stdout2 = String::from_utf8_lossy(&output2.stdout);

        // Extract added/removed counts
        fn extract_count(s: &str, pattern: &str) -> Option<usize> {
            let parts: Vec<&str> = s.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == pattern {
                    return parts.get(i - 1).and_then(|n| n.parse().ok());
                }
            }
            None
        }

        let added1 = extract_count(&stdout1, "added,").unwrap_or(0);
        let removed1 = extract_count(&stdout1, "removed,").unwrap_or(0);
        let added2 = extract_count(&stdout2, "added,").unwrap_or(0);
        let removed2 = extract_count(&stdout2, "removed,").unwrap_or(0);

        // Added in one direction should equal removed in the other
        prop_assert_eq!(added1, removed2, "added1 should equal removed2");
        prop_assert_eq!(removed1, added2, "removed1 should equal added2");
    }
}
