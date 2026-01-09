use jsondiff::diff::{
    engine::DiffEngine,
    types::{ArrayCompareMode, DiffConfig, DiffOp},
};
use proptest::prelude::*;
use sonic_rs::Value;

// ============================================================================
// Property Test Strategies
// ============================================================================

/// Generate arbitrary JSON-like values as sonic_rs Values
fn arb_json_primitive() -> impl Strategy<Value = Value> {
    prop_oneof![
        Just(Value::default()), // null
        any::<bool>().prop_map(|b| sonic_rs::json!(b)),
        any::<i32>().prop_map(|n| sonic_rs::json!(n)),
        any::<f64>()
            .prop_filter("finite float", |f| f.is_finite())
            .prop_map(|f| sonic_rs::json!(f)),
        "[a-zA-Z0-9_ ]{0,20}".prop_map(|s| sonic_rs::json!(s)),
    ]
}

/// Generate simple JSON objects as sonic_rs Values
fn arb_simple_json_object() -> impl Strategy<Value = Value> {
    prop::collection::vec(
        (
            "[a-z]{1,10}",
            prop_oneof![
                Just(Value::default()), // null
                any::<bool>().prop_map(|b| sonic_rs::json!(b)),
                any::<i32>().prop_map(|n| sonic_rs::json!(n)),
                "[a-zA-Z0-9]{0,10}".prop_map(|s| sonic_rs::json!(s)),
            ],
        ),
        0..5,
    )
    .prop_map(|pairs| {
        let mut obj = sonic_rs::Object::new();
        for (k, v) in pairs {
            obj.insert(&k, v);
        }
        Value::from(obj)
    })
}

/// Generate simple JSON arrays as sonic_rs Values
fn arb_simple_json_array() -> impl Strategy<Value = Value> {
    prop::collection::vec(arb_json_primitive(), 0..10).prop_map(|items| {
        let arr: sonic_rs::Array = items.into_iter().collect();
        Value::from(arr)
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

fn diff_with_config(left: &Value, right: &Value, config: DiffConfig) -> Vec<DiffOp> {
    let engine = DiffEngine::new(config);
    engine.diff(left, right).operations
}

fn diff_default(left: &Value, right: &Value) -> Vec<DiffOp> {
    diff_with_config(left, right, DiffConfig::default())
}

fn diff_set_mode(left: &Value, right: &Value) -> Vec<DiffOp> {
    diff_with_config(
        left,
        right,
        DiffConfig {
            array_mode: ArrayCompareMode::Set,
            ..Default::default()
        },
    )
}

fn diff_multiset_mode(left: &Value, right: &Value) -> Vec<DiffOp> {
    diff_with_config(
        left,
        right,
        DiffConfig {
            array_mode: ArrayCompareMode::MultiSet,
            ..Default::default()
        },
    )
}

fn count_ops(ops: &[DiffOp]) -> (usize, usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;
    for op in ops {
        match op {
            DiffOp::Added { .. } => added += 1,
            DiffOp::Removed { .. } => removed += 1,
            DiffOp::Modified { .. } => modified += 1,
        }
    }
    (added, removed, modified)
}

// ============================================================================
// Property Tests: Reflexivity
// ============================================================================

proptest! {
    /// Diffing a value against itself should always produce no differences
    #[test]
    fn prop_diff_self_is_empty(json in arb_simple_json_object()) {
        let ops = diff_default(&json, &json);
        prop_assert!(ops.is_empty(), "Expected no differences for identical JSON, got {:?}", ops);
    }

    /// Diffing an array against itself should always produce no differences
    #[test]
    fn prop_array_diff_self_is_empty(json in arb_simple_json_array()) {
        let ops = diff_default(&json, &json);
        prop_assert!(ops.is_empty(), "Expected no differences for identical array, got {:?}", ops);
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
        let arr1: sonic_rs::Array = elements.iter().map(|n| sonic_rs::json!(n)).collect();
        let mut reversed = elements.clone();
        reversed.reverse();
        let arr2: sonic_rs::Array = reversed.iter().map(|n| sonic_rs::json!(n)).collect();

        let json1 = Value::from(arr1);
        let json2 = Value::from(arr2);

        let ops = diff_set_mode(&json1, &json2);
        prop_assert!(ops.is_empty(),
            "Expected no differences in set mode for {:?} vs {:?}, got {:?}", json1, json2, ops);
    }

    /// In multiset mode, arrays with same element counts should be equal
    #[test]
    fn prop_array_multiset_count_based(
        elements in prop::collection::vec(1..5i32, 1..8)
    ) {
        let arr1: sonic_rs::Array = elements.iter().map(|n| sonic_rs::json!(n)).collect();
        let mut sorted = elements.clone();
        sorted.sort();
        sorted.reverse();
        let arr2: sonic_rs::Array = sorted.iter().map(|n| sonic_rs::json!(n)).collect();

        let json1 = Value::from(arr1);
        let json2 = Value::from(arr2);

        let ops = diff_multiset_mode(&json1, &json2);
        prop_assert!(ops.is_empty(),
            "Expected no differences in multiset mode for {:?} vs {:?}, got {:?}", json1, json2, ops);
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
        let arr1: sonic_rs::Array = elements1.iter().map(|n| sonic_rs::json!(n)).collect();
        let arr2: sonic_rs::Array = elements2.iter().map(|n| sonic_rs::json!(n)).collect();

        let json1 = Value::from(arr1);
        let json2 = Value::from(arr2);

        let ops1 = diff_set_mode(&json1, &json2);
        let ops2 = diff_set_mode(&json2, &json1);

        let (added1, removed1, _) = count_ops(&ops1);
        let (added2, removed2, _) = count_ops(&ops2);

        // Added in one direction should equal removed in the other
        prop_assert_eq!(added1, removed2, "added1 ({}) should equal removed2 ({})", added1, removed2);
        prop_assert_eq!(removed1, added2, "removed1 ({}) should equal added2 ({})", removed1, added2);
    }
}
