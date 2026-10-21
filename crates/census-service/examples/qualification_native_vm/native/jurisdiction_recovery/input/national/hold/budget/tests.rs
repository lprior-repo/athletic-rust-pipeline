use super::{held, preserved, validate, Transition};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

fn row(stage: &str, pauses: u32) -> Value {
    json!({"entry_id":"inv_original","entry_kind":"invocation","vqueue_id":"vq_owned",
        "deployment":"dp_owned","stage":stage,"retry_attempts":2,
        "retry_count_since_last_stored_command":1,"num_errors":1,"num_attempts":3,
        "num_pauses":pauses,"num_suspensions":1,"num_yields":1})
}

fn set(row: &mut Value, field: &str, value: Value) -> Result<()> {
    row.as_object_mut()
        .context("fixture malformed")?
        .insert(field.to_owned(), value);
    Ok(())
}

#[test]
fn quiescent_pause_preserves_actual_nonzero_retry_metadata() -> Result<()> {
    preserved(&row("suspended", 0), &row("paused", 1), Transition::Pause)?;
    held(&row("paused", 1), &row("paused", 1))?;
    let mut after = row("finished", 1);
    set(&mut after, "num_attempts", json!(4))?;
    preserved(&row("paused", 1), &after, Transition::Resume)?;
    Ok(())
}

#[test]
fn missing_null_string_or_changed_retry_values_refuse_instead_of_defaulting() -> Result<()> {
    for field in [
        "retry_attempts",
        "retry_count_since_last_stored_command",
        "num_errors",
    ] {
        let mut missing = row("paused", 1);
        missing
            .as_object_mut()
            .context("fixture malformed")?
            .remove(field);
        ensure!(
            preserved(&row("suspended", 0), &missing, Transition::Pause).is_err(),
            "missing {field} accepted"
        );
        for value in [
            Value::Null,
            json!("0"),
            json!(0),
            json!(99),
            json!(4294967296u64),
        ] {
            let mut changed = row("paused", 1);
            set(&mut changed, field, value)?;
            ensure!(
                preserved(&row("suspended", 0), &changed, Transition::Pause).is_err(),
                "forged {field} accepted"
            );
        }
    }
    Ok(())
}

#[test]
fn foreign_or_unquiesced_entries_and_extra_attempts_do_not_qualify() -> Result<()> {
    for (field, value) in [
        ("entry_id", json!("inv_other")),
        ("entry_kind", json!("state-mutation")),
        ("deployment", json!("dp_other")),
        ("vqueue_id", json!("vq_other")),
        ("num_attempts", json!(4)),
        ("num_pauses", json!(0)),
    ] {
        let mut after = row("paused", 1);
        set(&mut after, field, value)?;
        ensure!(
            preserved(&row("suspended", 0), &after, Transition::Pause).is_err(),
            "forged {field} accepted"
        );
    }
    ensure!(
        preserved(&row("running", 0), &row("paused", 1), Transition::Pause).is_err(),
        "active invoker pause qualified"
    );
    ensure!(
        validate(&row("paused", 1), "inv_other").is_err(),
        "foreign query result accepted"
    );
    let mut changed = row("paused", 1);
    set(&mut changed, "num_suspensions", json!(2))?;
    ensure!(
        held(&row("paused", 1), &changed).is_err(),
        "held metadata mutation accepted"
    );
    Ok(())
}
