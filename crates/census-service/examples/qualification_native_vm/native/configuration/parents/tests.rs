use super::unchanged;
use anyhow::Result;
use serde_json::{json, Value};

fn service() -> Value {
    json!({"name":"NationalCensus","deployment_id":"dp_owned","revision":1,
        "inactivity_timeout":"12h","abort_timeout":"12h","journal_retention":"90d",
        "idempotency_retention":"30d","retry_policy":{"max_attempts":3,"on_max_attempts":"Pause"},
        "handlers":[{"name":"run","ty":"Workflow"}],"public":true})
}

#[test]
fn isolated_parent_override_changes_only_inactivity_before_acceptance() -> Result<()> {
    let before = service();
    let mut after = before.clone();
    after
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("fixture malformed"))?
        .insert("inactivity_timeout".to_owned(), json!("1s"));
    unchanged(&before, &after)?;
    Ok(())
}

#[test]
fn isolated_parent_override_refuses_policy_pin_or_identity_changes() -> Result<()> {
    for (field, value) in [
        ("abort_timeout", json!("1s")),
        ("retry_policy", json!({"max_attempts":4})),
        ("deployment_id", json!("dp_foreign")),
        ("journal_retention", json!("1h")),
        ("idempotency_retention", json!("1h")),
        ("handlers", json!([])),
        ("public", json!(false)),
    ] {
        let before = service();
        let mut after = before.clone();
        let object = after
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("fixture malformed"))?;
        object.insert("inactivity_timeout".to_owned(), json!("1s"));
        object.insert(field.to_owned(), value);
        anyhow::ensure!(
            unchanged(&before, &after).is_err(),
            "changed {field} accepted"
        );
    }
    Ok(())
}

#[test]
fn parent_override_accepts_only_handler_order_differences() -> Result<()> {
    let mut before = service();
    let object = before
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("fixture malformed"))?;
    object.insert(
        "handlers".to_owned(),
        json!([
            {"name":"run","ty":"Workflow","input_json_schema":{"type":"object"}},
            {"name":"request","ty":"Shared","output_json_schema":{"type":"object"}}
        ]),
    );
    let mut after = before.clone();
    let object = after
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("fixture malformed"))?;
    object.insert("inactivity_timeout".to_owned(), json!("1s"));
    object
        .get_mut("handlers")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow::anyhow!("fixture handlers absent"))?
        .reverse();
    unchanged(&before, &after)?;
    for handlers in [
        json!([{"name":"run","ty":"Workflow"},{"name":"request","ty":"Shared"}]),
        json!([{"name":"run","ty":"Workflow"},{"name":"run","ty":"Workflow"}]),
        json!([{"name":"run","ty":"Workflow"},{"ty":"Shared"}]),
        json!([{"name":"run","ty":"Workflow"},{"name":"","ty":"Shared"}]),
    ] {
        let mut changed = after.clone();
        changed
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("fixture malformed"))?
            .insert("handlers".to_owned(), handlers);
        anyhow::ensure!(
            unchanged(&before, &changed).is_err(),
            "changed or ambiguous handler accepted"
        );
    }
    Ok(())
}
