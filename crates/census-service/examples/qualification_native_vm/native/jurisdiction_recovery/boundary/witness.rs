use super::{journal, observe, Observation, Original};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::{TeamsAttemptProgress, TeamsSourceInspection};
use serde_json::{json, Value};

pub(super) fn source_witness(
    observation: &Observation,
    parent_entries: &[Value],
    original: &Original,
    call: &super::SourceCall,
) -> Result<Option<Value>> {
    if !journal::invocation_ack(parent_entries, call)? {
        return Ok(None);
    }
    let Some(child_status) = observe::find_status(&observation.before, &call.child_id)? else {
        return Ok(None);
    };
    if super::input::text(child_status, "status")? != "running"
        || !super::journal_started(child_status)?
    {
        return Ok(None);
    }
    super::identity(
        child_status,
        &call.child_id,
        &call.child_key,
        "TeamsSource",
        original,
    )?;
    ensure!(
        super::input::text(child_status, "invoked_by_id")? == original.id
            && super::input::text(child_status, "invoked_by")? == "service",
        "source child is not owned by original parent"
    );
    let child = observation
        .children
        .iter()
        .find(|child| child.id == call.child_id)
        .context("observed source child absent")?;
    ensure!(child.key == call.child_key, "source unit key differs");
    let Some(attempt) = source_attempt(child)? else {
        return Ok(None);
    };
    let Some(child_entries) = journal::complete(&child.journal, child_status, &call.child_id)?
    else {
        return Ok(None);
    };
    journal::child_input(child_entries, call)?;
    Ok(Some(json!({
        "boundary":"active_unfinished_production_teams_source",
        "source_call":call,"source_attempt":attempt,
        "source_inspection":child.inspection_after,
        "source_store_journal_references":journal::source_ledger(&call.child_key, &child.inspection_after)?,
        "parent_pinned_deployment":original.deployment.get("id"),
        "child_pinned_deployment":child_status.get("pinned_deployment_id"),
        "child_input":journal::reference(child_entries.first().context("source Input absent")?)?,
        "reached_at":observation.clock,
        "physical_request_in_flight_proven":false,
        "unfinished_obligations":super::super::recovery::obligations(observation, original)?
    })))
}

fn source_attempt(child: &observe::Child) -> Result<Option<u8>> {
    if serde_json::to_value(&child.inspection_before)?
        != serde_json::to_value(&child.inspection_after)?
    {
        return Ok(None);
    }
    let TeamsSourceInspection::Unsettled { progress } = &child.inspection_after else {
        return Ok(None);
    };
    super::validate_progress(progress)?;
    Ok(match progress.last() {
        Some(TeamsAttemptProgress::Unknown { attempt }) => Some(*attempt),
        _ => None,
    })
}
