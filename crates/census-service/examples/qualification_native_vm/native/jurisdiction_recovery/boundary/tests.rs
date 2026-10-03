use super::super::observe::Observation;
use super::{assess, validate_progress};
use anyhow::{Context, Result};
use census_service::restate_services::{TeamsAttemptProgress, TeamsSourceInspection};
use serde_json::{json, Value};

mod support;
mod transitions;

#[test]
fn reserved_running_source_requires_unfinished_parent_call() -> Result<()> {
    let (original, observation) = support::active()?;
    let proof = assess(&observation, &original)?.context("active source boundary refused")?;
    anyhow::ensure!(
        proof.get("source_attempt") == Some(&json!(1)),
        "reservation ordinal not retained"
    );
    let mut completed = serde_json::to_value(&observation)?;
    completed.get_mut("parent_journal").and_then(Value::as_array_mut).context("journal absent")?.push(support::entry("inv_parent", 3, "Notification: Call", json!({"Notification":{"Completion":{"Call":{"completion_id":2,"result":{"Success":[]}}}}})));
    support::journal_size(&mut completed, "inv_parent", 4)?;
    anyhow::ensure!(
        assess(&serde_json::from_value(completed)?, &original)?.is_none(),
        "completed source call accepted as in flight"
    );
    Ok(())
}

#[test]
fn queued_child_and_uncertain_inspection_do_not_prove_active_source() -> Result<()> {
    let (original, observation) = support::active()?;
    let mut queued = serde_json::to_value(&observation)?;
    support::status_field(&mut queued, "inv_child", "status", json!("ready"))?;
    anyhow::ensure!(
        assess(&serde_json::from_value(queued)?, &original)?.is_none(),
        "queued source child counted as active"
    );
    let mut changed: Observation = serde_json::from_value(serde_json::to_value(observation)?)?;
    changed
        .children
        .first_mut()
        .context("source child absent")?
        .inspection_after = TeamsSourceInspection::Unsettled {
        progress: Vec::new(),
    };
    anyhow::ensure!(
        assess(&changed, &original)?.is_none(),
        "nonmatching source inspection bookends counted as boundary"
    );
    Ok(())
}

#[test]
fn foreign_source_child_and_changed_request_are_refused() -> Result<()> {
    let (original, observation) = support::active()?;
    let mut foreign = serde_json::to_value(&observation)?;
    support::status_field(
        &mut foreign,
        "inv_child",
        "invoked_by_id",
        json!("inv_replacement"),
    )?;
    anyhow::ensure!(
        assess(&serde_json::from_value(foreign)?, &original).is_err(),
        "foreign parent source child accepted"
    );
    let mut changed = serde_json::to_value(&observation)?;
    let entries = changed
        .get_mut("parent_journal")
        .and_then(Value::as_array_mut)
        .context("parent journal absent")?;
    let command = entries.get_mut(1).context("source command absent")?;
    let mut input = original.request.clone();
    input.refresh = true;
    let payload = census_service::restate_services::TeamsSourceRequest {
        jurisdiction: input,
        source: "milesplit".to_owned(),
        observed_on: "2026-10-02".to_owned(),
    };
    *command
        .pointer_mut("/entry_json/Command/Call/request/parameter")
        .context("source parameter absent")? = json!(serde_json::to_vec(&payload)?);
    anyhow::ensure!(
        assess(&serde_json::from_value(changed)?, &original).is_err(),
        "changed child request accepted as same run"
    );
    Ok(())
}

#[test]
fn missing_journal_row_and_wrong_awaited_completion_are_not_boundaries() -> Result<()> {
    let (original, observation) = support::active()?;
    let mut missing = serde_json::to_value(&observation)?;
    missing
        .get_mut("parent_journal")
        .and_then(Value::as_array_mut)
        .context("parent journal absent")?
        .pop()
        .context("journal row absent")?;
    anyhow::ensure!(
        assess(&serde_json::from_value(missing)?, &original)?.is_none(),
        "incomplete journal counted as witness"
    );
    let mut wrong = serde_json::to_value(&observation)?;
    support::status_field(
        &mut wrong,
        "inv_parent",
        "suspended_waiting_future_json",
        json!({"Single":{"CompletionId":99}}),
    )?;
    anyhow::ensure!(
        assess(&serde_json::from_value(wrong)?, &original)?.is_none(),
        "unrelated awaited completion counted as source boundary"
    );
    Ok(())
}

#[test]
fn running_parent_requires_recorded_unfinished_source_future() -> Result<()> {
    let (original, observation) = support::active()?;
    let mut running = serde_json::to_value(&observation)?;
    support::status_field(&mut running, "inv_parent", "status", json!("running"))?;
    support::status_field(
        &mut running,
        "inv_parent",
        "last_awaiting_on_future_json",
        json!({"Single":{"CompletionId":2}}),
    )?;
    anyhow::ensure!(
        assess(&serde_json::from_value(running.clone())?, &original)?.is_some(),
        "observed running parent source future refused"
    );
    support::status_field(
        &mut running,
        "inv_parent",
        "last_awaiting_on_future_json",
        Value::Null,
    )?;
    anyhow::ensure!(
        assess(&serde_json::from_value(running)?, &original)?.is_none(),
        "running parent with no source future counted as boundary"
    );
    Ok(())
}

#[test]
fn source_reservation_ordinal_and_attempt_ceiling_are_checked() -> Result<()> {
    validate_progress(&[TeamsAttemptProgress::Unknown { attempt: 1 }])?;
    anyhow::ensure!(
        validate_progress(&[TeamsAttemptProgress::Unknown { attempt: 2 }]).is_err(),
        "missing first reservation accepted"
    );
    let attempts = (1..=4)
        .map(|attempt| TeamsAttemptProgress::Unknown { attempt })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        validate_progress(&attempts).is_err(),
        "fourth source attempt accepted"
    );
    Ok(())
}
