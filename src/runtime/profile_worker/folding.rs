use super::super::{
    acquisition::{ProfileAcquisition, ProfileJob, TeamRequest},
    protocol::{DocumentReceipt, FailureCode, OperationFailure, SourceResource},
    Runtime,
};
use super::{
    intake::{acquire, initial_resources, retry_evidence},
    parsing::{parse_sources, parse_teams, ParsedTeam},
    reporting::{failure, issue},
    state::{absorb_initial, finalize_profile, BuildState},
    team::{self, TeamObservation},
};
use anyhow::Result;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(super) async fn initial_phase(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    job: &ProfileJob,
) -> Result<BuildState, HandlerError> {
    let resources = initial_resources(job.athlete_id)?;
    let fetched = acquire(ctx, &job.snapshot, resources).await?;
    let operations = fetched
        .iter()
        .map(|(_, outcome)| retry_evidence(outcome))
        .collect();
    let parsed = parse_sources(runtime, job.athlete_id, fetched).await;
    parsed.into_iter().try_fold(
        BuildState {
            complete: true,
            operations,
            ..BuildState::default()
        },
        absorb_initial,
    )
}

pub(super) async fn team_phase(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    job: &ProfileJob,
    mut state: BuildState,
) -> Result<ProfileAcquisition, HandlerError> {
    let (requests, exceeded) = team::unique_requests(state.requests);
    if exceeded {
        state.complete = false;
        state.issues.push(issue(
            "team_request_limit_exceeded",
            "authorized TeamNav requests exceeded the bounded acquisition limit",
            &job.snapshot,
            "/allSeasons",
        ));
    }
    let resources = team_resources(requests);
    let fetched = acquire(ctx, &job.snapshot, resources).await?;
    state
        .operations
        .extend(fetched.iter().map(|(_, outcome)| retry_evidence(outcome)));
    let parsed = parse_teams(runtime, fetched).await;
    let observations = absorb_teams(
        &mut state.responses,
        &mut state.failures,
        &mut state.complete,
        parsed,
    )?;
    let profile = finalize_profile(
        state.profiles,
        state.html,
        observations,
        state.issues,
        &mut state.failures,
        &mut state.complete,
    )?;
    Ok(ProfileAcquisition {
        athlete_id: job.athlete_id,
        profile,
        responses: state.responses,
        operations: state.operations,
        failures: state.failures,
        complete: state.complete,
    })
}

fn team_resources(requests: Vec<TeamRequest>) -> Vec<SourceResource> {
    requests
        .into_iter()
        .map(|request| SourceResource::Team {
            team_id: request.team_id,
            sport: request.sport,
            season: request.season,
        })
        .collect()
}

fn absorb_teams(
    responses: &mut Vec<DocumentReceipt>,
    failures: &mut Vec<OperationFailure>,
    complete: &mut bool,
    parsed: Vec<ParsedTeam>,
) -> Result<Vec<TeamObservation>, HandlerError> {
    parsed
        .into_iter()
        .try_fold(Vec::new(), |mut observations, item| {
            responses.extend(item.responses.clone());
            if let Some(message) = item.failure {
                *complete = false;
                failures.push(failure(
                    FailureCode::MalformedResponse,
                    message,
                    item.responses.clone(),
                )?);
            }
            if let Some(source_failure) = item.source_failure {
                *complete = false;
                failures.push(source_failure);
            }
            if let Some(observation) = item.observation {
                observations.push(observation);
            }
            Ok::<_, HandlerError>(observations)
        })
}
