use super::super::super::http::rows;
use super::super::boundary;
use super::super::journal::{self, SourceCall};
use super::super::observe::{self, Observation};
use super::{input, Original};
use anyhow::{ensure, Context, Result};
use serde_json::Value;

pub(super) fn boundary(value: &Value) -> Result<(SourceCall, Observation)> {
    let call = serde_json::from_value(
        value
            .pointer("/witness/source_call")
            .context("original source call absent")?
            .clone(),
    )?;
    let observation = serde_json::from_value(
        value
            .get("observation")
            .context("original boundary observation absent")?
            .clone(),
    )?;
    Ok((call, observation))
}

pub(super) fn held(
    original: &Original,
    call: &SourceCall,
    previous: &Observation,
    current: &Observation,
) -> Result<()> {
    binding(original, call, current)?;
    let parent = observe::status(&current.after, &original.id)?;
    ensure!(
        input::text(parent, "status")? == "paused",
        "original jurisdiction parent is not paused"
    );
    ensure!(
        current.parent_state.identity == original.key
            && current.parent_state.history_window == previous.parent_state.history_window
            && current.parent_state.history_window == Some(original.request.history)
            && current.parent_state.plan == previous.parent_state.plan,
        "held parent scope or plan changed"
    );
    prefix(&previous.parent_journal, &current.parent_journal)?;
    ensure!(
        previous.children.len() == current.children.len(),
        "held parent source child set changed"
    );
    previous.children.iter().try_for_each(|old| -> Result<()> {
        let now = current
            .children
            .iter()
            .find(|child| child.id == old.id && child.key == old.key)
            .context("held parent original source child disappeared")?;
        prefix(&old.journal, &now.journal)?;
        let old_status = observe::status(&previous.after, &old.id)?;
        let now_status = observe::status(&current.after, &old.id)?;
        for field in [
            "id",
            "pinned_deployment_id",
            "pinned_service_protocol_version",
        ] {
            let prior = old_status.get(field).filter(|value| !value.is_null());
            ensure!(
                prior.is_none_or(|value| now_status.get(field) == Some(value)),
                "held source changed {field}"
            );
        }
        Ok(())
    })
}

fn binding(original: &Original, call: &SourceCall, current: &Observation) -> Result<()> {
    ensure!(
        call.request.source == "milesplit"
            && call.child_key == format!("{}/teams/milesplit", original.key)
            && serde_json::to_value(&call.request.jurisdiction)?
                == serde_json::to_value(&original.request)?,
        "owned admission does not use the original MileSplit teams request"
    );
    input::safe_id(&call.child_id)?;
    let parent = observe::status(&current.after, &original.id)?;
    boundary::identity(
        parent,
        &original.id,
        &original.key,
        "JurisdictionCensus",
        original,
    )?;
    let entries = journal::complete(&current.parent_journal, parent, &original.id)?
        .context("original parent journal snapshot incomplete")?;
    journal::parent_input(entries, original)?;
    let actual = journal::calls(entries, original)?
        .into_iter()
        .find(|actual| actual.child_id == call.child_id)
        .context("original source call disappeared")?;
    ensure!(
        serde_json::to_value(actual)? == serde_json::to_value(call)?,
        "original source request changed"
    );
    let child = observe::status(&current.after, &call.child_id)?;
    boundary::identity(
        child,
        &call.child_id,
        &call.child_key,
        "TeamsSource",
        original,
    )?;
    ensure!(
        input::text(child, "invoked_by_id")? == original.id
            && child
                .get("pinned_service_protocol_version")
                .and_then(Value::as_u64)
                == Some(7),
        "original source parent or protocol differs"
    );
    Ok(())
}

pub(super) fn prefix(before: &Value, after: &Value) -> Result<()> {
    let before = rows(before)?;
    let after = rows(after)?;
    ensure!(
        before.len() <= after.len() && after.len() <= 2048,
        "original journal prefix length differs"
    );
    before
        .iter()
        .zip(after)
        .enumerate()
        .try_for_each(|(index, (old, current))| -> Result<()> {
            ensure!(
                old.get("index").and_then(Value::as_u64) == Some(u64::try_from(index)?),
                "original journal prefix is not ordered"
            );
            for field in ["id", "index", "version", "entry_type", "entry_json"] {
                let retained = old
                    .get(field)
                    .with_context(|| format!("original journal {field} absent"))?;
                ensure!(
                    current.get(field) == Some(retained),
                    "original journal {field} changed"
                );
            }
            Ok(())
        })
}

#[cfg(test)]
mod tests;
