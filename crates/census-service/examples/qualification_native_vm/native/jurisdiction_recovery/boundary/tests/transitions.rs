use super::super::super::observe::Observation;
use super::super::assess;
use super::support;
use anyhow::{Context, Result};
use serde_json::{json, Value};

#[test]
fn unjournaled_source_waits_until_same_running_child_is_pinned() -> Result<()> {
    let (original, active) = support::active()?;
    ["pending", "ready", "running"]
        .into_iter()
        .try_for_each(|status| -> Result<()> {
            let mut pending = serde_json::to_value(&active)?;
            support::status_field(&mut pending, "inv_child", "status", json!(status))?;
            support::status_field(
                &mut pending,
                "inv_child",
                "pinned_deployment_id",
                Value::Null,
            )?;
            support::status_field(
                &mut pending,
                "inv_child",
                "pinned_service_protocol_version",
                Value::Null,
            )?;
            support::journal_size(&mut pending, "inv_child", 0)?;
            *pending
                .pointer_mut("/children/0/journal")
                .context("child journal absent")? = json!([]);
            let pending: Observation = serde_json::from_value(pending)?;
            assert_eq!(assess(&pending, &original)?, None);
            let transitioning = Observation {
                after: active.after.clone(),
                ..pending
            };
            assert_eq!(assess(&transitioning, &original)?, None);
            Ok(())
        })?;
    let proof = assess(&active, &original)?.context("pinned running source boundary absent")?;
    assert_eq!(
        proof.pointer("/source_call/child_id"),
        Some(&json!("inv_child"))
    );
    assert_eq!(
        proof.get("child_pinned_deployment"),
        Some(&json!("dp_owned"))
    );
    assert_eq!(
        proof.pointer("/child_input/invocation_id"),
        Some(&json!("inv_child"))
    );
    Ok(())
}

#[test]
fn absent_invocations_are_pending_observations_not_boundaries() -> Result<()> {
    let (original, active) = support::active()?;
    ["inv_parent", "inv_child"]
        .into_iter()
        .try_for_each(|id| -> Result<()> {
            let mut observation = serde_json::to_value(&active)?;
            ["before", "after"]
                .into_iter()
                .try_for_each(|bookend| -> Result<()> {
                    observation
                        .get_mut(bookend)
                        .and_then(Value::as_array_mut)
                        .context("status rows absent")?
                        .retain(|row| row.get("id").and_then(Value::as_str) != Some(id));
                    Ok(())
                })?;
            assert_eq!(
                assess(&serde_json::from_value(observation)?, &original)?,
                None
            );
            Ok(())
        })
}

#[test]
fn missing_or_foreign_pin_after_journal_start_is_refused() -> Result<()> {
    let (original, active) = support::active()?;
    [
        ("pinned_deployment_id", Value::Null),
        ("pinned_deployment_id", json!("dp_foreign")),
        ("pinned_service_protocol_version", Value::Null),
        ("pinned_service_protocol_version", json!(3)),
        ("restarted_from", json!("inv_previous")),
    ]
    .into_iter()
    .try_for_each(|(field, value)| -> Result<()> {
        let mut observation = serde_json::to_value(&active)?;
        support::status_field(&mut observation, "inv_child", field, value)?;
        assess(&serde_json::from_value(observation)?, &original)
            .err()
            .context("invalid source binding accepted")?;
        Ok(())
    })
}

#[test]
fn unstarted_parent_without_pin_does_not_require_journal_binding() -> Result<()> {
    let (original, active) = support::active()?;
    ["pending", "running"]
        .into_iter()
        .try_for_each(|status| -> Result<()> {
            let mut observation = serde_json::to_value(&active)?;
            support::status_field(&mut observation, "inv_parent", "status", json!(status))?;
            support::status_field(
                &mut observation,
                "inv_parent",
                "pinned_deployment_id",
                Value::Null,
            )?;
            support::status_field(
                &mut observation,
                "inv_parent",
                "pinned_service_protocol_version",
                Value::Null,
            )?;
            support::journal_size(&mut observation, "inv_parent", 0)?;
            *observation
                .get_mut("parent_journal")
                .context("parent journal absent")? = json!([]);
            assert_eq!(
                assess(&serde_json::from_value(observation)?, &original)?,
                None
            );
            Ok(())
        })
}

#[test]
fn duplicate_original_child_is_not_an_absent_observation() -> Result<()> {
    let (original, active) = support::active()?;
    let mut observation = serde_json::to_value(&active)?;
    ["before", "after"]
        .into_iter()
        .try_for_each(|bookend| -> Result<()> {
            let rows = observation
                .get_mut(bookend)
                .and_then(Value::as_array_mut)
                .context("status rows absent")?;
            let child = rows
                .iter()
                .find(|row| row.get("id") == Some(&json!("inv_child")))
                .context("child absent")?
                .clone();
            rows.push(child);
            Ok(())
        })?;
    let error = assess(&serde_json::from_value(observation)?, &original)
        .err()
        .context("duplicate child accepted")?;
    assert_eq!(
        error.to_string(),
        "original invocation inv_child duplicated"
    );
    Ok(())
}

#[test]
fn unassigned_running_child_waits_but_malformed_journal_count_is_refused() -> Result<()> {
    let (original, active) = support::active()?;
    let mut observation = serde_json::to_value(&active)?;
    ["before", "after"]
        .into_iter()
        .try_for_each(|bookend| -> Result<()> {
            let row = observation
                .get_mut(bookend)
                .and_then(Value::as_array_mut)
                .context("status rows absent")?
                .iter_mut()
                .find(|row| row.get("id") == Some(&json!("inv_child")))
                .context("child absent")?
                .as_object_mut()
                .context("child status malformed")?;
            row.remove("pinned_deployment_id");
            row.remove("pinned_service_protocol_version");
            row.remove("journal_size");
            Ok(())
        })?;
    *observation
        .pointer_mut("/children/0/journal")
        .context("child journal absent")? = json!([]);
    assert_eq!(
        assess(&serde_json::from_value(observation.clone())?, &original)?,
        None
    );
    support::status_field(&mut observation, "inv_child", "journal_size", Value::Null)?;
    assert_eq!(
        assess(&serde_json::from_value(observation.clone())?, &original)?,
        None
    );
    support::status_field(
        &mut observation,
        "inv_child",
        "journal_size",
        json!("unknown"),
    )?;
    let error = assess(&serde_json::from_value(observation)?, &original)
        .err()
        .context("malformed journal count accepted")?;
    assert_eq!(error.to_string(), "journal size malformed");
    Ok(())
}
