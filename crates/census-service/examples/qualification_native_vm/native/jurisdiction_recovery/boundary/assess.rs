use super::{journal, observe, Observation, Original};
use anyhow::{ensure, Result};
use census_service::restate_services::TeamsStage;
use serde_json::Value;

pub(super) fn assess_source(
    observation: &Observation,
    original: &Original,
    operation: Option<&str>,
) -> Result<Option<Value>> {
    if observation.before != observation.after {
        return Ok(None);
    }
    let Some(parent) = observe::find_status(&observation.before, &original.id)? else {
        return Ok(None);
    };
    if !matches!(
        super::input::text(parent, "status")?,
        "suspended" | "running"
    ) || !super::journal_started(parent)?
    {
        return Ok(None);
    }
    let Some(entries) = journal::complete(&observation.parent_journal, parent, &original.id)?
    else {
        return Ok(None);
    };
    if entries.is_empty() {
        return Ok(None);
    }
    super::identity(
        parent,
        &original.id,
        &original.key,
        "JurisdictionCensus",
        original,
    )?;
    journal::parent_input(entries, original)?;
    let calls = journal::calls(entries, original)?;
    if calls.is_empty() {
        return Ok(None);
    }
    let Some(plan) = observation.parent_state.plan.as_ref() else {
        return Ok(None);
    };
    validate_plan(observation, original, plan)?;
    let completions = journal::completions(entries)?;
    let awaited = journal::awaited(parent)?;
    calls.iter().try_fold(None, |witness, call| {
        if witness.is_some()
            || completions.contains(&call.result_completion)
            || !awaited.contains(&call.result_completion)
            || operation.is_some_and(|operation| operation != call.child_key)
        {
            return Ok(witness);
        }
        super::witness::source_witness(observation, entries, original, call)
    })
}

fn validate_plan(
    observation: &Observation,
    original: &Original,
    plan: &census_service::restate_services::SourcePlan,
) -> Result<()> {
    ensure!(
        observation.parent_state.identity == original.key
            && plan.sweepable == original.source_plan.sweepable
            && !plan.fingerprint.is_empty(),
        "parent source plan identity differs"
    );
    ensure!(
        matches!(&observation.parent_state.teams, TeamsStage::Owed),
        "source boundary does not retain owed teams stage: observed {}",
        stage_name(&observation.parent_state.teams)
    );
    Ok(())
}

fn stage_name(teams: &TeamsStage) -> &'static str {
    match teams {
        TeamsStage::Owed => "owed",
        TeamsStage::Completed(_) => "completed",
        TeamsStage::Failed(_) => "failed",
    }
}
