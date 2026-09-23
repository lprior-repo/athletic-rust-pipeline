//! Waiting for a run: the polling loop, the jurisdiction probe, and the progress line an operator
//! reads while it runs.

use anyhow::{bail, Result};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use midwest_census::census::{Revision, WorkflowIdentity};
use midwest_census::restate_services::{
    JurisdictionCensusIngressClient, JurisdictionReport, JurisdictionRequest, NationalReport,
};
use restate_sdk::ingress::{InvocationHandle, Output, ReqwestClient, SendStatus};
use restate_sdk::prelude::*;

use super::{POLL, PROGRESS_EVERY};
use crate::cli::ingress;

/// The pieces a waiting national command needs to observe one run. The invocation handle carries
/// the ingress client the output reads go through, so it is the only transport needed here.
pub(crate) struct Watch<'a> {
    pub(crate) handle: &'a InvocationHandle<reqwest::Client, Json<NationalReport>>,
    pub(crate) ingestion: &'a ReqwestClient,
    pub(crate) jurisdictions: &'a [UsJurisdiction],
    pub(crate) season: SchoolYear,
    pub(crate) revision: Revision,
    pub(crate) rounds: u64,
}

/// Wait for the run's output, printing how each jurisdiction is doing along the way.
///
/// The bound is the command's `--timeout-seconds`. Reaching it is not a cancellation: the run keeps
/// going, and rerunning the same command reattaches, which is why the message says so instead of
/// implying the census failed.
pub(crate) async fn observe(watch: Watch<'_>) -> Result<NationalReport> {
    let mut round: u64 = 0;
    while round < watch.rounds {
        round = round.saturating_add(1);
        match watch
            .handle
            .output()
            .await
            .map_err(ingress::error)?
            .into_body()
        {
            Output::Ready(Json(report)) => return Ok(report),
            Output::NotReady => {}
        }
        if round.checked_rem(PROGRESS_EVERY) == Some(0) {
            print_progress(
                watch.ingestion,
                watch.jurisdictions,
                watch.season,
                watch.revision,
            )
            .await;
        }
        tokio::time::sleep(POLL).await;
    }
    bail!(
        "observation bound of {} seconds reached; invocation {} was not cancelled — rerun the same command to reattach",
        watch.rounds.saturating_mul(POLL.as_secs()),
        watch.handle.invocation_id()
    )
}

/// Submit one jurisdiction's durable run and wait for its report.
///
/// Shared by the commands that drive a state's census in one submission: the object's key is the
/// jurisdiction identity, so a repeat submission joins the run that already holds it instead of
/// starting a second one. The submission line prints for every state, including the ones Restate
/// deduplicated, because "your submission was not applied" is a fact the operator has to read.
pub(crate) async fn drive_jurisdiction(
    ingestion: &ReqwestClient,
    request: JurisdictionRequest,
    rounds: u64,
) -> Result<JurisdictionReport> {
    let identity =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
    let object = JurisdictionCensusIngressClient::from_client(ingestion.clone(), identity.as_str());
    let submitted = object
        .run(Json(request))
        .send()
        .await
        .map_err(ingress::error)?;
    let handle = submitted.invocation_handle();
    println!(
        "{} submitted as invocation {}",
        identity.as_str(),
        handle.invocation_id()
    );
    if matches!(submitted.send_status(), SendStatus::PreviouslyAccepted) {
        println!(
            "note: {} already had a run — restate deduplicated this submission",
            identity.as_str()
        );
    }
    observe_jurisdiction(&handle, rounds).await
}

/// Wait for one jurisdiction's run the same way the national command waits for the fan-out.
pub(crate) async fn observe_jurisdiction(
    handle: &InvocationHandle<reqwest::Client, Json<JurisdictionReport>>,
    rounds: u64,
) -> Result<JurisdictionReport> {
    let mut round: u64 = 0;
    while round < rounds {
        round = round.saturating_add(1);
        match handle.output().await.map_err(ingress::error)?.into_body() {
            Output::Ready(Json(report)) => return Ok(report),
            Output::NotReady => {}
        }
        tokio::time::sleep(POLL).await;
    }
    bail!(
        "observation bound of {} seconds reached; invocation {} was not cancelled — rerun the same command to reattach",
        rounds.saturating_mul(POLL.as_secs()),
        handle.invocation_id()
    )
}

/// One progress line: how many jurisdictions have finished each stage, and the cohort totals they
/// report so far.
///
/// A jurisdiction whose state cannot be read is counted, not fatal. It is normally a state that has
/// not been reached yet — the object exists only once the fan-out calls it — and the operator asked
/// for the run's progress, which is still true.
pub(crate) async fn print_progress(
    ingestion: &ReqwestClient,
    jurisdictions: &[UsJurisdiction],
    season: SchoolYear,
    revision: Revision,
) {
    let mut teams_done: usize = 0;
    let mut rosters_done: usize = 0;
    let mut athletes: usize = 0;
    let mut class_of_2027: usize = 0;
    let mut unreadable: usize = 0;
    for jurisdiction in jurisdictions {
        let identity = WorkflowIdentity::jurisdiction(*jurisdiction, season, revision);
        let object =
            JurisdictionCensusIngressClient::from_client(ingestion.clone(), identity.as_str());
        let read = object
            .state()
            .call()
            .await
            .map(|response| response.into_body());
        let Ok(Ok(Json(state))) = read else {
            unreadable = unreadable.saturating_add(1);
            continue;
        };
        if state.teams.is_some() {
            teams_done = teams_done.saturating_add(1);
        }
        if let Some(progress) = &state.rosters {
            rosters_done = rosters_done.saturating_add(1);
            athletes = athletes.saturating_add(progress.athletes);
            class_of_2027 = class_of_2027.saturating_add(progress.class_of_2027);
        }
    }
    println!(
        "progress: teams {teams_done}/{} · rosters {rosters_done}/{} · athletes {athletes} · co2027 {class_of_2027} · unstarted {unreadable}",
        jurisdictions.len(),
        jurisdictions.len(),
    );
}
