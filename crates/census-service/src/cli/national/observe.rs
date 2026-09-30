use anyhow::{bail, Result};
use census_crawl::ingress::{CurrentRoute, Ingress};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_service::restate_services::{
    JurisdictionCensusIngressClient, JurisdictionReport, JurisdictionRequest, NationalReport,
};
use restate_sdk::ingress::{InvocationHandle, Output, SendStatus};
use restate_sdk::prelude::*;

use super::{POLL, PROGRESS_EVERY};
use census_service::ingress;

pub(crate) struct Watch<'a> {
    pub(crate) handle: &'a InvocationHandle<CurrentRoute, Json<NationalReport>>,
    pub(crate) ingestion: &'a Ingress,
    pub(crate) jurisdictions: &'a [UsJurisdiction],
    pub(crate) season: SchoolYear,
    pub(crate) revision: Revision,
    pub(crate) rounds: u64,
}

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

pub(crate) async fn drive_jurisdiction(
    ingestion: &Ingress,
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

pub(crate) async fn observe_jurisdiction(
    handle: &InvocationHandle<CurrentRoute, Json<JurisdictionReport>>,
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

pub(crate) async fn print_progress(
    ingestion: &Ingress,
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
