use super::raises;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

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

fn refusal(before: Value, after: Value) -> Result<Vec<String>> {
    raises(
        &BTreeMap::new(),
        &scan_report(after),
        &baseline_report(before),
    )
}

#[test]
fn an_oversized_file_swapped_for_another_is_a_raise() -> TestResult {
    let raised = refusal(
        json!(["census-service:crates/census-service/src/old.rs (412)"]),
        json!(["census-service:crates/census-service/src/new.rs (398)"]),
    )?;

    check!(eq; raised.len(), 1, "{raised:?}");
    let first = raised
        .first()
        .ok_or_else(|| anyhow::anyhow!("missing refusal: {raised:?}"))?;
    check!(first.contains("src/new.rs"), "{raised:?}");
    check!(
        first.contains("structure files_over_300_lines"),
        "{raised:?}"
    );
    Ok(())
}

#[test]
fn an_oversized_file_that_grew_is_a_raise_and_one_that_shrank_is_not() -> TestResult {
    let path = "census-service:crates/census-service/src/store/mod.rs";

    let raised = refusal(
        json!([format!("{path} (312)")]),
        json!([format!("{path} (360)")]),
    )?;
    check!(eq; raised.len(), 1, "{raised:?}");
    let first = raised
        .first()
        .ok_or_else(|| anyhow::anyhow!("missing refusal: {raised:?}"))?;
    check!(first.contains("312 -> 360"), "{raised:?}");

    let quiet = refusal(
        json!([format!("{path} (312)")]),
        json!([format!("{path} (300)")]),
    )?;
    check!(quiet.is_empty(), "{quiet:?}");
    Ok(())
}
