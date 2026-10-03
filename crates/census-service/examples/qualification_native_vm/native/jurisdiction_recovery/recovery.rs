use super::super::http::{request, INGRESS};
use super::input::{self, Original};
use super::journal;
use super::observe::{self, Observation};
use super::{boundary, captures};
use anyhow::{ensure, Context, Result};
use census_service::restate_services::{TeamsSourceInspection, TeamsSourceOutcome, TeamsStage};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::time::Duration;

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
    let recovered = wait(client, original).await?;
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
    let parent = observe::status(&recovered.after, &original.id)?;
    let evidence = json!({"original":original,"before":before,"recovered":recovered,"sources":sources,"captures":retained,"capture_obligations":capture_obligations,"unfinished_obligations":obligations(&recovered, original)?,"clock":clock,"same_original_recovered":true,"replacement_submitted":false,"physical_request_in_flight_proven":false,"national_pass":false});
    observe::publish("recovery-reconciliation", &evidence)?;
    ensure!(input::text(parent, "status")? == "completed", "original remains unfinished after bounded recovery; exact obligations retained in jurisdiction-recovery-recovery-reconciliation.json");
    ensure!(
        input::text(parent, "completion_result")? == "success",
        "original recovered with explicit terminal failure: {}",
        parent
            .get("completion_failure")
            .context("terminal failure reason absent")?
    );
    ensure!(capture_obligations.as_array().is_some_and(Vec::is_empty), "completed source has unproven physical capture obligations; retained reconciliation is not acquisition PASS");
    let output = request(
        client,
        Method::GET,
        &format!("{INGRESS}restate/attach/{}", original.id),
        None,
    )
    .await?;
    let report: census_service::restate_services::JurisdictionReport =
        serde_json::from_value(output.clone())?;
    ensure!(
        report.identity == original.key && report.jurisdiction == original.request.jurisdiction,
        "reattached output identity differs"
    );
    Ok(
        json!({"reconciliation":evidence,"original_output":output,"reattached_invocation_id":original.id,"source_stage_recovery_observed":true,"national_pass":false,"unproven_obligations":["HTTP request/response/capture/parse fault subphase has no production signal", "physical Fjall source entity/effect receipt readback requires the offline single-owner oracle", "archive URL/time association is not a capture-to-child journal reference"]}),
    )
}

#[tracing::instrument(skip(client, original))]
async fn wait(client: &Client, original: &Original) -> Result<Observation> {
    let checks = stream::iter(0..300_u32).then(|attempt| async move {
        let statuses = observe::statuses(client, &original.id).await?;
        observe::publish(
            "recovery-status-latest",
            &json!({"attempt":attempt,"original_id":original.id,"statuses":statuses}),
        )?;
        let parent = observe::status(&statuses, &original.id)?;
        if input::text(parent, "status")? == "completed" || attempt == 299 {
            let observation = observe::read(client, original).await?;
            observe::publish("recovery-latest", &json!({"attempt":attempt,"observation":observation,"unfinished_obligations":obligations(&observation, original)?}))?;
            let recovered_parent = observe::status(&observation.after, &original.id)?;
            if (input::text(recovered_parent, "status")? == "completed"
                && observation.before == observation.after)
                || attempt == 299
            {
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

fn source_reconciliation(original: &Original, observation: &Observation) -> Result<Value> {
    let parent = observe::status(&observation.after, &original.id)?;
    let entries = journal::complete(&observation.parent_journal, parent, &original.id)?
        .context("parent journal changed during recovery observation")?;
    let calls = journal::calls(entries, original)?;
    let completions = journal::completions(entries)?;
    let sources = calls.iter().map(|call| -> Result<Value> {
        let status = observe::status(&observation.after, &call.child_id)?;
        boundary::identity(status, &call.child_id, &call.child_key, "TeamsSource", original)?;
        ensure!(input::text(status, "invoked_by_id")? == original.id, "retained child parent changed");
        let child = observation.children.iter().find(|child| child.id == call.child_id).context("original source child missing after reboot")?;
        ensure!(serde_json::to_value(&child.inspection_before)? == serde_json::to_value(&child.inspection_after)?, "source inspection changed during recovery bookends");
        let entries = journal::complete(&child.journal, status, &call.child_id)?.context("source child journal changed during recovery observation")?;
        journal::child_input(entries, call)?;
        let progress = progress(&child.inspection_after);
        boundary::validate_progress(progress)?;
        Ok(json!({"source":call.request.source,"source_unit":call.child_key,"original_child_id":call.child_id,"original_call":call,"status":status,"inspection":child.inspection_after,"source_store_journal_references":journal::source_ledger(&call.child_key, &child.inspection_after)?,"parent_call_result_recorded":completions.contains(&call.result_completion),"retained_journal":child.journal}))
    }).collect::<Result<Vec<_>>>()?;
    ensure!(
        observation.children.len() == sources.len(),
        "unmatched source child in retained original tree"
    );
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
        "meets":if state.meets.is_some() { "stage_returned_inspect_outcome" } else { "owed" },
        "results":if state.results.is_some() { "stage_returned_inspect_outcome" } else { "owed" },
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
