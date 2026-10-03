use super::{retained_source_binding, SourceBinding};
use anyhow::{Context, Result};
use serde_json::{json, Value};

fn status(state: &str, deployment: Value, protocol: Value, journal_size: u64) -> Value {
    json!({
        "id":"inv_sibling",
        "target_service_name":"TeamsSource",
        "target_service_key":"original/teams/riil",
        "target_handler_name":"run",
        "status":state,
        "restarted_from":null,
        "pinned_deployment_id":deployment,
        "pinned_service_protocol_version":protocol,
        "journal_size":journal_size
    })
}

fn binding(previous: &Value, current: &Value) -> Result<SourceBinding> {
    retained_source_binding(
        previous,
        current,
        "inv_sibling",
        "original/teams/riil",
        "dp_owned",
    )
}

#[test]
fn pending_sibling_first_pin_is_accepted_without_certifying_unassigned_state() -> Result<()> {
    let pending = status("pending", Value::Null, Value::Null, 0);
    let started = status("running", json!("dp_owned"), json!(7), 1);
    assert_eq!(binding(&pending, &pending)?, SourceBinding::Unassigned);
    assert_eq!(binding(&pending, &started)?, SourceBinding::Pinned);
    Ok(())
}

#[test]
fn started_sibling_cannot_change_or_erase_observed_deployment() -> Result<()> {
    let previous = status("running", json!("dp_owned"), json!(7), 1);
    [json!("dp_foreign"), Value::Null]
        .into_iter()
        .try_for_each(|pin| -> Result<()> {
            let current = status("running", pin, json!(7), 2);
            let error = binding(&previous, &current)
                .err()
                .context("changed existing pin accepted")?;
            assert_eq!(
                error.to_string(),
                "original source child pinned_deployment_id changed across reboot"
            );
            Ok(())
        })
}

#[test]
fn observed_protocol_is_immutable_even_before_journal_size_is_visible() -> Result<()> {
    let previous = status("running", Value::Null, json!(7), 0);
    let current = status("running", json!("dp_owned"), json!(6), 1);
    let error = binding(&previous, &current)
        .err()
        .context("changed observed protocol accepted")?;
    assert_eq!(
        error.to_string(),
        "original source child pinned_service_protocol_version changed across reboot"
    );
    Ok(())
}

#[test]
fn first_pin_must_be_actual_retained_deployment_and_supported_protocol() -> Result<()> {
    let previous = status("pending", Value::Null, Value::Null, 0);
    [
        (
            json!("dp_foreign"),
            json!(7),
            "source invocation deployment is not the retained deployment",
        ),
        (
            Value::Null,
            json!(7),
            "required pinned_deployment_id absent",
        ),
        (
            json!("dp_owned"),
            Value::Null,
            "source invocation protocol outside journal-v2 scope",
        ),
        (
            json!("dp_owned"),
            json!(8),
            "source invocation protocol outside journal-v2 scope",
        ),
    ]
    .into_iter()
    .try_for_each(|(pin, protocol, expected)| -> Result<()> {
        let current = status("running", pin, protocol, 1);
        let error = binding(&previous, &current)
            .err()
            .context("invalid first pin accepted")?;
        assert_eq!(error.to_string(), expected);
        Ok(())
    })
}

#[test]
fn pending_sibling_cannot_be_replaced_while_waiting_for_first_pin() -> Result<()> {
    let previous = status("pending", Value::Null, Value::Null, 0);
    let mut replacement = previous.clone();
    *replacement
        .get_mut("restarted_from")
        .context("restart field absent")? = json!("inv_previous");
    let error = binding(&previous, &replacement)
        .err()
        .context("replacement sibling accepted")?;
    assert_eq!(
        error.to_string(),
        "replacement invocation cannot prove original recovery"
    );
    Ok(())
}
