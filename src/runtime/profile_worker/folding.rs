use super::super::{
    acquisition::{ProfileAcquisition, ProfileJob},
    protocol::{FailureCode, SourceResource},
    Runtime,
};
use super::{
    intake::{acquire, initial_resources, retry_evidence},
    parsing::{parse_sources, parse_teams},
    reporting::{failure, issue},
    state::{absorb_initial, finalize_profile, BuildState},
    team,
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
    let resources = requests
        .into_iter()
        .map(|request| SourceResource::Team {
            team_id: request.team_id,
            sport: request.sport,
            season: request.season,
        })
        .collect();
    let fetched = acquire(ctx, &job.snapshot, resources).await?;
    state
        .operations
        .extend(fetched.iter().map(|(_, outcome)| retry_evidence(outcome)));
    let parsed = parse_teams(runtime, fetched).await;
    let observations = parsed
        .into_iter()
        .try_fold(Vec::new(), |mut observations, item| {
            state.responses.extend(item.responses.clone());
            if let Some(message) = item.failure {
                state.complete = false;
                state.failures.push(failure(
                    FailureCode::MalformedResponse,
                    message,
                    item.responses.clone(),
                )?);
            }
            if let Some(source_failure) = item.source_failure {
                state.complete = false;
                state.failures.push(source_failure);
            }
            if let Some(observation) = item.observation {
                observations.push(observation);
            }
            Ok::<_, HandlerError>(observations)
        })?;
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
