use super::captures;
use super::input::{self, Original};
use super::journal;
use super::observe::{self, Observation};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::{TeamsSourceInspection, TeamsSourceOutcome, TeamsStage};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

mod parent;
mod quiescence;
mod reconcile;

#[tracing::instrument(skip(client, original, before, clock))]
pub(super) async fn finish(
    client: &Client,
    original: &Original,
    before: &Value,
    clock: Value,
) -> Result<Value> {
    let initial = observe::read(client, original).await?;
    reconcile::original(original, before, &initial)?;
    observe::publish(
        "recovery-reattached",
        &json!({"original_id":original.id,"observation":initial,"clock":clock,"replacement_submitted":false}),
    )?;
    let candidate = wait(client, original).await?;
    let (recovered, control) = quiescence::hold(client, original, candidate).await?;
    reconcile::original(original, before, &recovered)?;
    let sources = source_reconciliation(original, &recovered)?;
    let retained = captures::snapshot(original.key.clone()).await?;
    captures::reconcile(&original.captures_before, &retained)?;
    let boundary_captures: Vec<captures::CaptureRef> = serde_json::from_value(
        before
            .pointer("/boundary/captures")
            .context("pre-reset capture references absent")?
            .clone(),
    )?;
    captures::reconcile(&boundary_captures, &retained)?;
    let capture_obligations = reconcile::capture_obligations(original, &retained, &sources)?;
    let evidence = json!({"original":original,"before":before,"recovered":recovered,"sources":sources,"captures":retained,"capture_obligations":capture_obligations,"unfinished_obligations":obligations(&recovered, original)?,"clock":clock,"same_original_recovered":true,"replacement_submitted":false,"physical_request_in_flight_proven":false,"national_pass":false});
    observe::publish("recovery-reconciliation", &evidence)?;
    ensure!(capture_obligations.as_array().is_some_and(Vec::is_empty), "completed source has unproven physical capture obligations; retained reconciliation is not acquisition PASS");
    let parent_recovery = parent::outcome(client, original, &recovered, control).await?;
    Ok(
        json!({"reconciliation":evidence,"parent_recovery":parent_recovery,"reattached_invocation_id":original.id,"source_stage_recovery_observed":true,"national_pass":false,"unproven_obligations":["HTTP request/response/capture/parse fault subphase has no production signal", "archive URL/time association is not a capture-to-child journal reference"]}),
    )
}

const CHECKS: u32 = 3_200;

#[tracing::instrument(skip(client, original))]
async fn wait(client: &Client, original: &Original) -> Result<Observation> {
    let checks = stream::iter(0..CHECKS).then(|attempt| async move {
        let statuses = observe::statuses(client, &original.id).await?;
        observe::publish(
            "recovery-status-latest",
            &json!({"attempt":attempt,"original_id":original.id,"statuses":statuses}),
        )?;
        let parent = observe::status(&statuses, &original.id)?;
        let source_calls_settled = super::super::http::rows(&statuses)?.iter()
            .filter(|row| row.get("target_service_name").and_then(Value::as_str) == Some("TeamsSource"))
            .all(|row| row.get("status").and_then(Value::as_str) == Some("completed"));
        if input::text(parent, "status")? == "completed" || source_calls_settled || attempt == CHECKS - 1 {
            let observation = observe::read(client, original).await?;
            observe::publish("recovery-latest", &json!({"attempt":attempt,"observation":observation,"unfinished_obligations":obligations(&observation, original)?}))?;
            if candidate(original, &observation)? {
                return Ok(Some(observation));
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok::<_, anyhow::Error>(None)
    }).try_filter_map(|value| futures::future::ready(Ok(value)));
    futures::pin_mut!(checks);
    checks
        .try_next()
        .await?
        .context("recovery observation budget exhausted")
}

fn candidate(original: &Original, observation: &Observation) -> Result<bool> {
    Ok(journal::settled_sources(original, observation)?.is_some())
}

pub(super) fn verify_finished(finished: &Value, original: &Original) -> Result<()> {
    let observation: Observation = serde_json::from_value(
        finished
            .pointer("/reconciliation/recovered")
            .context("recovered observation absent")?
            .clone(),
    )?;
    let parent: parent::ParentRecovery = serde_json::from_value(
        finished
            .get("parent_recovery")
            .context("explicit parent recovery disposition absent")?
            .clone(),
    )?;
    parent.verify(original, &observation)?;
    ensure!(
        candidate(original, &observation)?,
        "original settled source result reconciliation is unproven"
    );
    Ok(())
}

#[tracing::instrument(skip(client, original, finished))]
pub(super) async fn verify_live(
    client: &Client,
    original: &Original,
    finished: &Value,
) -> Result<Value> {
    verify_finished(finished, original)?;
    let previous: Observation = serde_json::from_value(
        finished
            .pointer("/reconciliation/recovered")
            .context("recovered observation absent")?
            .clone(),
    )?;
    let current = observe::read(client, original).await?;
    let parent: parent::ParentRecovery = serde_json::from_value(
        finished
            .get("parent_recovery")
            .context("explicit parent recovery disposition absent")?
            .clone(),
    )?;
    parent.verify(original, &current)?;
    ensure!(
        previous.parent_journal == current.parent_journal
            && serde_json::to_value(previous.parent_state)?
                == serde_json::to_value(&current.parent_state)?,
        "original parent journal or remaining state changed during quiescent interval"
    );
    ensure!(
        source_reconciliation(original, &current)?
            == *finished
                .pointer("/reconciliation/sources")
                .context("retained source reconciliation absent")?,
        "original source journal or progress changed during quiescent interval"
    );
    let tree = parent.live_tree(client, original).await?;
    Ok(
        json!({"original_invocation_id":original.id,"parent_recovery":parent,"invocation_tree":tree,"quiescent":true,"replacement_submitted":false}),
    )
}

fn source_reconciliation(original: &Original, observation: &Observation) -> Result<Value> {
    let sources = journal::settled_sources(original, observation)?
        .context("original source bijection or settled evidence is unproven")?
        .into_iter()
        .map(|source| -> Result<Value> {
            let call = source.call;
            let child = source.child;
            let status = source.status;
            Ok(json!({"source":call.request.source,"source_unit":call.child_key,"original_child_id":call.child_id,"original_call":call,"status":status,"inspection":child.inspection_after,"source_store_journal_references":journal::source_ledger(&call.child_key, &child.inspection_after)?,"parent_call_result_recorded":true,"retained_journal":child.journal}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!(sources))
}

pub(super) fn obligations(observation: &Observation, original: &Original) -> Result<Value> {
    let state = &observation.parent_state;
    let plan = state.plan.as_ref();
    let source_units = plan.map_or(original.source_plan.sweepable.as_slice(), |plan| plan.sweepable.as_slice()).iter().map(|slug| {
        let key = format!("{}/teams/{slug}", original.key);
        let retained = observation.children.iter().find(|child| child.key == key);
        let disposition = match retained.map(|child| &child.inspection_after) {
            Some(TeamsSourceInspection::Settled { outcome: TeamsSourceOutcome::Completed { .. } }) => "completed",
            Some(TeamsSourceInspection::Settled { .. }) => "terminal_or_interrupted",
            Some(TeamsSourceInspection::Unsettled { .. }) => "unfinished_reserved_or_unstarted",
            None if slug == "milesplit" || slug == "riil" => "not_yet_called",
            None => "non_teams_source_stage",
        };
        json!({"source":slug,"source_unit":key,"disposition":disposition,"inspection":retained.map(|child| &child.inspection_after)})
    }).collect::<Vec<_>>();
    Ok(json!({
        "source_units":source_units,
        "teams":match &state.teams { TeamsStage::Owed => "owed", TeamsStage::Completed(_) => "completed", TeamsStage::Failed(_) => "failed_with_retained_reasons" },
        "teams_evidence":state.teams,
        "rosters":if state.rosters.is_some() { "stage_returned_inspect_outcome" } else { "owed" },
        "rosters_evidence":state.rosters,
        "original_request":original.request,
        "retained_parent_journal":observation.parent_journal,
        "original_parent_status":observe::status(&observation.after, &original.id)?,
        "history":state.history,
        "history_window":state.history_window,
        "source_refusals":plan.map(|plan| &plan.refused),
        "remaining_scope":"RI qualification; one roster target; not national coverage"
    }))
}

pub(super) fn progress(
    inspection: &TeamsSourceInspection,
) -> &[census_service::restate_services::TeamsAttemptProgress] {
    match inspection {
        TeamsSourceInspection::Unsettled { progress } => progress,
        TeamsSourceInspection::Settled { outcome } => match outcome {
            TeamsSourceOutcome::Completed { progress, .. }
            | TeamsSourceOutcome::Terminal { progress, .. }
            | TeamsSourceOutcome::Exhausted { progress, .. }
            | TeamsSourceOutcome::Interrupted { progress, .. } => progress,
        },
    }
}
