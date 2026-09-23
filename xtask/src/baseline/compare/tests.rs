//! The refusal's own judgement: what `xtask quality-baseline update` will not record.
//!
//! Only the oversized-file ledger is exercised, because it is the one ledger the refusal reads by
//! path rather than by count: everything else it refuses on a monotone number.

use super::raises;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The smallest scan report the refusal reads — the ledger it is being asked about.
fn scan_report(oversized: Value) -> Value {
    json!({
        "crates": {},
        "structure": { "files_over_300_lines": oversized },
        "context": {},
    })
}

/// The smallest baseline the refusal compares against.
fn baseline_report(oversized: Value) -> Value {
    json!({ "structure": { "files_over_300_lines": oversized } })
}

/// What the refusal reports for `before` -> `after`.
fn refusal(before: Value, after: Value) -> Vec<String> {
    raises(
        &BTreeMap::new(),
        &scan_report(after),
        &baseline_report(before),
    )
    .expect("the fixture report carries `crates` and `structure`")
}

#[test]
fn an_oversized_file_swapped_for_another_is_a_raise() {
    // One file before and one after: counting the ledger would say the structure did not move, while
    // the file the baseline records stopped being measured.
    let raised = refusal(
        json!(["midwest-census:crates/midwest-census/src/old.rs (412)"]),
        json!(["midwest-census:crates/midwest-census/src/new.rs (398)"]),
    );

    assert_eq!(raised.len(), 1, "{raised:?}");
    assert!(raised[0].contains("src/new.rs"), "{raised:?}");
    assert!(raised[0].contains("structure files_over_300_lines"), "{raised:?}");
}

#[test]
fn an_oversized_file_that_grew_is_a_raise_and_one_that_shrank_is_not() {
    let path = "midwest-census:crates/midwest-census/src/store/mod.rs";

    let raised = refusal(
        json!([format!("{path} (312)")]),
        json!([format!("{path} (360)")]),
    );
    assert_eq!(raised.len(), 1, "{raised:?}");
    assert!(raised[0].contains("312 -> 360"), "{raised:?}");

    let quiet = refusal(
        json!([format!("{path} (312)")]),
        json!([format!("{path} (300)")]),
    );
    assert!(quiet.is_empty(), "{quiet:?}");
}
