use super::raises;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn scan_report(oversized: Value) -> Value {
    json!({
        "crates": {},
        "structure": { "files_over_300_lines": oversized },
        "context": {},
    })
}

fn baseline_report(oversized: Value) -> Value {
    json!({ "structure": { "files_over_300_lines": oversized } })
}

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
    let raised = refusal(
        json!(["census-service:crates/census-service/src/old.rs (412)"]),
        json!(["census-service:crates/census-service/src/new.rs (398)"]),
    );

    assert_eq!(raised.len(), 1, "{raised:?}");
    assert!(raised[0].contains("src/new.rs"), "{raised:?}");
    assert!(
        raised[0].contains("structure files_over_300_lines"),
        "{raised:?}"
    );
}

#[test]
fn an_oversized_file_that_grew_is_a_raise_and_one_that_shrank_is_not() {
    let path = "census-service:crates/census-service/src/store/mod.rs";

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
