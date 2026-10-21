use super::prefix;
use anyhow::{ensure, Result};
use serde_json::{json, Value};

fn entry(index: u64, body: &str) -> Value {
    json!({"id":"inv_original","index":index,"version":2,
        "entry_type":"Command: Run","entry_json":body})
}

#[test]
fn literal_journal_prefix_accepts_only_appended_entries() -> Result<()> {
    let before = json!([entry(0, "{ \"Command\": {\"Run\": {}} }")]);
    let after = json!([entry(0, "{ \"Command\": {\"Run\": {}} }"), entry(1, "{}")]);
    prefix(&before, &after)?;
    ensure!(
        prefix(&after, &before).is_err(),
        "journal truncation accepted"
    );
    Ok(())
}

#[test]
fn equal_decoded_json_cannot_replace_retained_literal_journal_spelling() -> Result<()> {
    let before = json!([entry(0, "{ \"Command\": {\"Run\": {}} }")]);
    let after = json!([entry(0, "{\"Command\":{\"Run\":{}}}")]);
    ensure!(
        prefix(&before, &after).is_err(),
        "literal source prefix was reserialized"
    );
    Ok(())
}

#[test]
fn journal_reference_changes_and_missing_fields_refuse() -> Result<()> {
    let before = json!([entry(0, "{}")]);
    for (field, value) in [
        ("id", json!("inv_replacement")),
        ("index", json!(1)),
        ("version", json!(1)),
        ("entry_type", json!("Notification: Call")),
    ] {
        let mut changed = entry(0, "{}");
        changed
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("fixture is not an object"))?
            .insert(field.to_owned(), value);
        ensure!(
            prefix(&before, &json!([changed])).is_err(),
            "changed {field} accepted"
        );
    }
    ensure!(
        prefix(&json!([{}]), &json!([{}])).is_err(),
        "incomplete journal accepted"
    );
    ensure!(
        prefix(&json!([entry(1, "{}")]), &json!([entry(1, "{}")])).is_err(),
        "unordered journal accepted"
    );
    Ok(())
}

#[test]
fn prefix_entry_budget_is_enforced_before_comparison() -> Result<()> {
    let after = (0..2049u64)
        .map(|index| entry(index, "{}"))
        .collect::<Vec<_>>();
    ensure!(
        prefix(&json!([]), &json!(after)).is_err(),
        "oversized journal accepted"
    );
    Ok(())
}
