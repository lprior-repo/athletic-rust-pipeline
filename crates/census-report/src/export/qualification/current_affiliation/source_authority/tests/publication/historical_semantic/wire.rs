use super::*;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Interpretation {
    schema: String,
    original_capture_interpretation: Box<RawValue>,
    semantic_inventory: Box<RawValue>,
}

pub(super) fn mutate(bytes: &[u8], mutation: Mutation) -> TestResult<Vec<u8>> {
    let mut interpretation: Interpretation = serde_json::from_slice(bytes)?;
    check!(eq; serde_json::to_vec(&interpretation)?, bytes);
    check!(eq; interpretation.schema, "roster-retained-capture-interpretation-v3");
    let original = interpretation.semantic_inventory.get();
    let mut changed: Value = serde_json::from_str(original)?;
    let rows = changed
        .pointer("/rows")
        .and_then(Value::as_array)
        .ok_or("rows array")?;
    check!(eq; rows.len(), 1);
    check!(eq; changed.pointer("/disposition").and_then(Value::as_str), Some("complete"));
    match mutation {
        Mutation::OmitMember => {
            changed
                .pointer_mut("/rows")
                .and_then(Value::as_array_mut)
                .ok_or("rows array")?
                .clear();
        }
        Mutation::ChangeName => {
            let row = changed.pointer_mut("/rows/0").ok_or("single row")?;
            row["provider_id"] = serde_json::json!("Forged Controlled Person");
        }
        Mutation::DuplicateMember => {
            let row = changed.pointer("/rows/0").ok_or("single row")?.clone();
            changed
                .pointer_mut("/rows")
                .and_then(Value::as_array_mut)
                .ok_or("rows array")?
                .push(row);
        }
    }
    let expected = match mutation {
        Mutation::OmitMember => 0,
        Mutation::ChangeName => 1,
        Mutation::DuplicateMember => 2,
    };
    check!(eq; changed.pointer("/rows").and_then(Value::as_array).ok_or("rows array")?.len(), expected);
    let changed = changed.to_string();
    check!(changed.as_str() != original);
    interpretation.semantic_inventory = serde_json::from_str(&changed)?;
    Ok(serde_json::to_vec(&interpretation)?)
}
