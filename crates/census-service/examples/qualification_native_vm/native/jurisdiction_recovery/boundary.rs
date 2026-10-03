use super::injection;
use super::input::{self, Original};
use super::journal::{self, SourceCall};
use super::observe::{self, Observation};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::{TeamsAttemptProgress, TeamsSourceInspection, TeamsStage};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

#[tracing::instrument(skip(client, original))]
pub(super) async fn wait(client: &Client, original: &Original) -> Result<Value> {
    let probes = stream::iter(0..120_u32).then(|attempt| async move {
        let observation = observe::read(client, original).await?;
        let witness = runtime_assess(&observation, original);
        let candidate = match witness {
            Ok(witness) => witness,
            Err(error) => {
                observe::publish("invalid-boundary", &json!({"attempt":attempt,"observation":observation,"error":format!("{error:#}")}))?;
                return Err(error);
            }
        };
        observe::publish("latest-probe", &json!({"attempt":attempt,"observation":observation,"witness":candidate}))?;
        if let Some(candidate) = candidate.as_ref() {
            match confirm(client, original, candidate).await {
                Ok(Some(confirmed)) => return Ok(Some(confirmed)),
                Ok(None) => {}
                Err(error) => {
                    observe::publish("invalid-boundary", &json!({"attempt":attempt,"observation":observation,"error":format!("{error:#}")}))?;
                    return Err(error);
                }
            }
        }
        let completed = observe::find_status(&observation.after, &original.id)?
            .map(|parent| input::text(parent, "status").map(|status| status == "completed"))
            .transpose()?.is_some_and(|completed| completed);
        if completed {
            let error = "original completed before fully published native reservation marker and active source boundary; reset refused";
            observe::publish("invalid-boundary", &json!({"attempt":attempt,"observation":observation,"error":error}))?;
            anyhow::bail!(error);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok::<_, anyhow::Error>(None)
    }).try_filter_map(|value| futures::future::ready(Ok(value)));
    futures::pin_mut!(probes);
    let reached = probes.try_next().await?;
    if reached.is_none() {
        observe::publish(
            "invalid-boundary",
            &json!({"error":"no fully published matching reservation marker and active original source witness reached in 120 bounded probes; QMP reset refused","operation":injection::operation(original)?,"latest_probe":"jurisdiction-recovery-latest-probe.json"}),
        )?;
    }
    reached.context("native reservation marker missing, mismatched or unreached at the active original source boundary; QMP reset refused")
}

#[tracing::instrument(skip(client, original))]
async fn confirm(client: &Client, original: &Original, initial: &Value) -> Result<Option<Value>> {
    let captures = super::captures::snapshot(original.key.clone()).await?;
    let observation = observe::read(client, original).await?;
    let witness = runtime_assess(&observation, original)?;
    if let Some(current) = witness.as_ref() {
        ensure!(
            initial.get("native_source_boundary") == current.get("native_source_boundary"),
            "native reservation marker/config changed during boundary confirmation"
        );
    }
    Ok(witness
        .map(|witness| json!({"observation":observation,"witness":witness,"captures":captures})))
}

fn runtime_assess(observation: &Observation, original: &Original) -> Result<Option<Value>> {
    let operation = injection::operation(original)?;
    assess_source(observation, original, Some(&operation))?
        .map(|witness| injection::witness(observation, original, witness))
        .transpose()
        .map(Option::flatten)
}

#[cfg(test)]
pub(super) fn assess(observation: &Observation, original: &Original) -> Result<Option<Value>> {
    assess_source(observation, original, None)
}

fn assess_source(
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
    if !matches!(input::text(parent, "status")?, "suspended" | "running")
        || !journal_started(parent)?
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
    identity(
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
        source_witness(observation, entries, original, call)
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
        "source boundary does not retain owed teams stage"
    );
    Ok(())
}

fn source_witness(
    observation: &Observation,
    parent_entries: &[Value],
    original: &Original,
    call: &SourceCall,
) -> Result<Option<Value>> {
    if !journal::invocation_ack(parent_entries, call)? {
        return Ok(None);
    }
    let Some(child_status) = observe::find_status(&observation.before, &call.child_id)? else {
        return Ok(None);
    };
    if input::text(child_status, "status")? != "running" || !journal_started(child_status)? {
        return Ok(None);
    }
    identity(
        child_status,
        &call.child_id,
        &call.child_key,
        "TeamsSource",
        original,
    )?;
    ensure!(
        input::text(child_status, "invoked_by_id")? == original.id
            && input::text(child_status, "invoked_by")? == "service",
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
        "unfinished_obligations":super::recovery::obligations(observation, original)?
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
    validate_progress(progress)?;
    Ok(match progress.last() {
        Some(TeamsAttemptProgress::Unknown { attempt }) => Some(*attempt),
        _ => None,
    })
}

pub(super) fn journal_started(status: &Value) -> Result<bool> {
    match status.get("journal_size") {
        None | Some(Value::Null) => Ok(false),
        Some(size) => Ok(size.as_u64().context("journal size malformed")? > 0),
    }
}

pub(super) fn identity(
    status: &Value,
    id: &str,
    key: &str,
    service: &str,
    original: &Original,
) -> Result<()> {
    target_identity(status, id, key, service)?;
    pinned_identity(status, input::text(&original.deployment, "id")?)
        .with_context(|| format!("original {service} invocation {id} pin evidence: {status}"))
}

pub(super) fn target_identity(status: &Value, id: &str, key: &str, service: &str) -> Result<()> {
    ensure!(
        input::text(status, "id")? == id
            && input::text(status, "target_service_key")? == key
            && input::text(status, "target_service_name")? == service
            && input::text(status, "target_handler_name")? == "run",
        "original invocation target identity mismatch"
    );
    ensure!(
        status.get("restarted_from").is_none_or(Value::is_null),
        "replacement invocation cannot prove original recovery"
    );
    Ok(())
}

pub(super) fn pinned_identity(status: &Value, deployment: &str) -> Result<()> {
    ensure!(
        input::text(status, "pinned_deployment_id")? == deployment,
        "source invocation deployment is not the retained deployment"
    );
    ensure!(
        status
            .get("pinned_service_protocol_version")
            .and_then(Value::as_u64)
            .is_some_and(|version| (4..=7).contains(&version)),
        "source invocation protocol outside journal-v2 scope"
    );
    Ok(())
}

pub(super) fn validate_progress(progress: &[TeamsAttemptProgress]) -> Result<()> {
    ensure!(
        progress.len() <= 3,
        "source physical-attempt ceiling exceeded"
    );
    progress
        .iter()
        .enumerate()
        .try_for_each(|(index, step)| -> Result<()> {
            let attempt = match step {
                TeamsAttemptProgress::Unknown { attempt }
                | TeamsAttemptProgress::Completed { attempt, .. }
                | TeamsAttemptProgress::Transient { attempt, .. }
                | TeamsAttemptProgress::Terminal { attempt, .. } => *attempt,
            };
            ensure!(
                usize::from(attempt) == index.checked_add(1).context("source ordinal overflow")?,
                "source reservation ordinals changed"
            );
            Ok(())
        })
}

#[cfg(test)]
mod tests;
