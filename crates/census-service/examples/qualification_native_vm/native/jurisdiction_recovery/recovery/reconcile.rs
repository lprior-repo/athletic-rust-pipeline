use super::super::super::super::{artifacts, GUEST};
use super::super::input::{self, Original};
use super::super::journal;
use super::super::observe::{self, Observation};
use super::super::{boundary, captures::CaptureRef};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::TeamsSourceOutcome;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn original(
    original: &Original,
    before: &Value,
    observation: &Observation,
) -> Result<()> {
    let root = Path::new(GUEST);
    ensure!(
        artifacts::json(&root.join("manifest.json"))? == original.manifest,
        "recovery immutable run manifest differs"
    );
    ensure!(
        artifacts::json(&root.join("deployment.json"))? == original.deployment,
        "recovery registration differs"
    );
    let parent = observe::status(&observation.after, &original.id)?;
    boundary::identity(
        parent,
        &original.id,
        &original.key,
        "JurisdictionCensus",
        original,
    )?;
    ensure!(
        observation.parent_state.identity == original.key,
        "retained parent state identity differs"
    );
    ensure!(
        observation
            .parent_state
            .history_window
            .is_none_or(|window| window == original.request.history),
        "retained historical window differs from original immutable request"
    );
    let previous: Observation = serde_json::from_value(
        before
            .pointer("/boundary/observation")
            .context("original boundary observation absent")?
            .clone(),
    )?;
    ensure!(
        observe::status(&previous.after, &original.id)?.get("pinned_service_protocol_version")
            == parent.get("pinned_service_protocol_version"),
        "original parent protocol changed across reboot"
    );
    retained_entries(&previous.parent_journal, &observation.parent_journal)?;
    let call: journal::SourceCall = serde_json::from_value(
        before
            .pointer("/boundary/witness/source_call")
            .context("original source call absent")?
            .clone(),
    )?;
    let child_status = observe::status(&observation.after, &call.child_id)?;
    boundary::identity(
        child_status,
        &call.child_id,
        &call.child_key,
        "TeamsSource",
        original,
    )?;
    ensure!(
        input::text(child_status, "invoked_by_id")? == original.id,
        "retained source child parent differs"
    );
    retained_children(original, &previous, observation)?;
    ensure!(
        previous.parent_state.plan.as_ref() == observation.parent_state.plan.as_ref(),
        "original source plan fingerprint changed across reboot"
    );
    Ok(())
}

fn retained_children(
    original: &Original,
    previous: &Observation,
    observation: &Observation,
) -> Result<()> {
    previous.children.iter().try_for_each(|old| -> Result<()> {
        let current = observation
            .children
            .iter()
            .find(|child| child.id == old.id && child.key == old.key)
            .context("original source child missing after reboot")?;
        let old_status = observe::status(&previous.after, &old.id)?;
        let current_status = observe::status(&observation.after, &old.id)?;
        retained_source_binding(
            old_status,
            current_status,
            &old.id,
            &old.key,
            input::text(&original.deployment, "id")?,
        )?;
        retained_entries(&old.journal, &current.journal)?;
        let old_progress = super::progress(&old.inspection_after);
        let current_progress = super::progress(&current.inspection_after);
        boundary::validate_progress(current_progress)?;
        ensure!(
            current_progress.len() >= old_progress.len(),
            "source reservations disappeared after reboot"
        );
        old_progress
            .iter()
            .zip(current_progress)
            .try_for_each(|(old, current)| -> Result<()> {
                if matches!(
                    old,
                    census_service::restate_services::TeamsAttemptProgress::Unknown { .. }
                ) {
                    return Ok(());
                }
                ensure!(
                    serde_json::to_value(old)? == serde_json::to_value(current)?,
                    "acknowledged source attempt changed after reboot"
                );
                Ok(())
            })
    })
}

#[derive(Debug, PartialEq, Eq)]
enum SourceBinding {
    Unassigned,
    Pinned,
}

fn retained_source_binding(
    previous: &Value,
    current: &Value,
    id: &str,
    key: &str,
    deployment: &str,
) -> Result<SourceBinding> {
    boundary::target_identity(current, id, key, "TeamsSource")?;
    if boundary::journal_started(previous)? {
        boundary::pinned_identity(previous, deployment)?;
    }
    let fields = ["pinned_deployment_id", "pinned_service_protocol_version"];
    let already_bound = fields
        .into_iter()
        .try_fold(false, |bound, field| -> Result<bool> {
            let prior = previous.get(field).filter(|value| !value.is_null());
            ensure!(
                prior.is_none_or(|value| current.get(field) == Some(value)),
                "original source child {field} changed across reboot"
            );
            Ok(bound || prior.is_some())
        })?;
    let now_bound = fields
        .into_iter()
        .any(|field| current.get(field).is_some_and(|value| !value.is_null()));
    if already_bound || now_bound || boundary::journal_started(current)? {
        boundary::pinned_identity(current, deployment)?;
        return Ok(SourceBinding::Pinned);
    }
    Ok(SourceBinding::Unassigned)
}

fn retained_entries(before: &Value, after: &Value) -> Result<()> {
    let before = super::super::super::http::rows(before)?;
    let after = super::super::super::http::rows(after)?;
    ensure!(
        before.len() <= 2048 && after.len() <= 2048,
        "retained journal exceeds evidence budget"
    );
    before.iter().try_for_each(|old| -> Result<()> {
        let current = after
            .iter()
            .find(|row| row.get("index") == old.get("index"))
            .context("acknowledged journal entry missing after reboot")?;
        ensure!(
            current.get("id") == old.get("id")
                && current.get("version") == old.get("version")
                && current.get("entry_type") == old.get("entry_type"),
            "retained original journal reference changed"
        );
        let old_entry = journal::decode(
            old.get("entry_json")
                .context("original journal body absent")?,
        )?;
        let current_entry = journal::decode(
            current
                .get("entry_json")
                .context("recovered journal body absent")?,
        )?;
        ensure!(
            old_entry == current_entry,
            "acknowledged journal command or completion changed after reboot"
        );
        Ok(())
    })
}

pub(super) fn retained_observation(
    original: &Original,
    previous: &Observation,
    current: &Observation,
) -> Result<()> {
    let old = observe::status(&previous.after, &original.id)?;
    let parent = observe::status(&current.after, &original.id)?;
    ensure!(
        old.get("pinned_service_protocol_version") == parent.get("pinned_service_protocol_version"),
        "original parent protocol changed during pause race"
    );
    ensure!(
        current.parent_state.identity == original.key
            && current.parent_state.history_window.is_none_or(|window| window == original.request.history)
            && previous.parent_state.plan == current.parent_state.plan,
        "original parent scope changed during pause race"
    );
    retained_entries(&previous.parent_journal, &current.parent_journal)?;
    retained_children(original, previous, current)
}

pub(super) fn capture_obligations(
    original: &Original,
    captures: &[CaptureRef],
    sources: &Value,
) -> Result<Value> {
    let started = chrono::DateTime::parse_from_rfc3339(input::text(&original.clock, "realtime")?)?;
    let obligations = sources.as_array().context("source reconciliation malformed")?.iter().try_fold(Vec::new(), |mut obligations, source| -> Result<Vec<Value>> {
        let outcome = source.pointer("/inspection/outcome");
        let Some(outcome) = outcome else { return Ok(obligations); };
        let outcome: TeamsSourceOutcome = serde_json::from_value(outcome.clone())?;
        if !matches!(outcome, TeamsSourceOutcome::Completed { .. }) { return Ok(obligations); }
        let slug = input::text(source, "source")?;
        let acquired = captures.iter().filter(|capture| capture.source == slug && !original.captures_before.contains(capture)).try_fold(false, |found, capture| -> Result<bool> {
            let fetched = chrono::DateTime::parse_from_rfc3339(input::text(&capture.metadata, "fetched_at")?)?;
            Ok(found || (fetched >= started && capture.bytes > 0 && capture.metadata.get("status").and_then(Value::as_u64) == Some(200)))
        })?;
        if !acquired {
            obligations.try_reserve(1)?;
            obligations.push(json!({"source_unit":source.get("source_unit"),"reason":"completed source has no verified nonempty fresh successful production archive capture; no external acquisition PASS"}));
        }
        Ok(obligations)
    })?;
    Ok(json!(obligations))
}

#[cfg(test)]
mod tests;
