use super::super::input::{self, Original};
use super::super::observe::{self, Child, Observation};
use super::super::{boundary, recovery};
use super::{SourceCall, child_input, complete, completions, parent_input};
use anyhow::{Context, Result, ensure};
use census_service::restate_services::TeamsSourceInspection;
use serde_json::Value;

pub(in super::super) struct SettledSource<'a> {
    pub(in super::super) call: SourceCall,
    pub(in super::super) child: &'a Child,
    pub(in super::super) status: &'a Value,
}

pub(in super::super) fn settled_sources<'a>(
    original: &Original,
    observation: &'a Observation,
) -> Result<Option<Vec<SettledSource<'a>>>> {
    if observation.before != observation.after || observation.children.is_empty() {
        return Ok(None);
    }
    let parent = observe::status(&observation.after, &original.id)?;
    boundary::identity(parent, &original.id, &original.key, "JurisdictionCensus", original)?;
    let Some(entries) = complete(&observation.parent_journal, parent, &original.id)? else {
        return Ok(None);
    };
    parent_input(entries, original)?;
    let calls = super::calls(entries, original)?;
    bijection(original, observation, &calls)?;
    let completions = completions(entries)?;
    calls.into_iter().try_fold(Some(Vec::new()), |sources, call| {
        let Some(mut sources) = sources else { return Ok(None); };
        let child = observation.children.iter().find(|child| child.id == call.child_id)
            .context("original source child missing after reboot")?;
        let status = observe::status(&observation.after, &call.child_id)?;
        if !completions.contains(&call.result_completion) || !settled(child, status, &call)? {
            return Ok(None);
        }
        sources.try_reserve(1)?;
        sources.push(SettledSource { call, child, status });
        Ok(Some(sources))
    })
}

fn bijection(original: &Original, observation: &Observation, calls: &[SourceCall]) -> Result<()> {
    ensure!(observation.children.len() == calls.len(), "unmatched original source calls or children");
    let rows = super::super::super::http::rows(&observation.after)?;
    ensure!(rows.len() == calls.len().checked_add(1).context("source count overflow")?,
        "unmatched source status in original tree");
    observation.children.iter().enumerate().try_for_each(|(index, child)| -> Result<()> {
        ensure!(!observation.children.iter().take(index).any(|prior| prior.id == child.id || prior.key == child.key),
            "duplicate observed source child identity");
        let call = calls.iter().find(|call| call.child_id == child.id)
            .context("orphan original source child")?;
        ensure!(child.key == call.child_key, "observed source child key differs from parent call");
        let status = observe::status(&observation.after, &child.id)?;
        boundary::identity(status, &child.id, &call.child_key, "TeamsSource", original)?;
        ensure!(input::text(status, "invoked_by_id")? == original.id, "retained child parent changed");
        Ok(())
    })
}

fn settled(child: &Child, status: &Value, call: &SourceCall) -> Result<bool> {
    if !matches!(&child.inspection_after, TeamsSourceInspection::Settled { .. })
        || serde_json::to_value(&child.inspection_before)? != serde_json::to_value(&child.inspection_after)?
        || input::text(status, "status")? != "completed"
        || input::text(status, "completion_result")? != "success"
    {
        return Ok(false);
    }
    let Some(entries) = complete(&child.journal, status, &child.id)? else {
        return Ok(false);
    };
    child_input(entries, call)?;
    boundary::validate_progress(recovery::progress(&child.inspection_after))?;
    Ok(true)
}
