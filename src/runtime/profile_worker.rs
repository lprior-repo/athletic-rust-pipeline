pub(crate) mod team;

use self::team::TeamObservation;
use super::{
    acquisition::{ProfileAcquisition, ProfileJob, ProfileProbe, TeamRequest},
    protocol::{
        DocumentReceipt, FailureCode, FetchOutcome, OperationFailure, RetryEvidence, SourceResource,
    },
    source_cache::{SourceCacheClient, SourceRequest},
    Runtime,
};
use crate::{
    domain::{
        evidence::{EvidenceIssue, EvidenceRef, Observed, ProfileEvidence, TeamEvidence},
        identity::{AthleteId, EvidenceDigest, ProfileUrl},
        name::BioIdentityObservation,
    },
    profile,
};
use anyhow::{Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use restate_sdk::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

const MAX_TEAM_REQUESTS: usize = 4_096;
const FETCH_CONCURRENCY: usize = 8;

pub struct ProfileWorker {
    pub runtime: Arc<Runtime>,
}

#[derive(Debug)]
enum Payload {
    Bio {
        profile: Box<ProfileEvidence>,
        identity: Option<BioIdentityObservation>,
        requests: Vec<TeamRequest>,
        issues: Vec<EvidenceIssue>,
    },
    Html(profile::HtmlProfileEvidence),
}

struct ParsedSource {
    responses: Vec<DocumentReceipt>,
    payload: Option<Payload>,
    failure: Option<String>,
    source_failure: Option<OperationFailure>,
}

#[derive(Debug, Default)]
struct BuildState {
    responses: Vec<DocumentReceipt>,
    operations: Vec<RetryEvidence>,
    failures: Vec<OperationFailure>,
    profiles: Vec<ProfileEvidence>,
    identities: Vec<BioIdentityObservation>,
    html: Option<profile::HtmlProfileEvidence>,
    requests: Vec<TeamRequest>,
    issues: Vec<EvidenceIssue>,
    complete: bool,
}

#[restate_sdk::object(
    ingress_private = true,
    inactivity_timeout = "2h",
    journal_retention = "30 days",
    idempotency_retention = "30 days",
    invocation_retry_policy(initial_interval = "1s", max_attempts = 4, on_max_attempts = "pause")
)]
impl ProfileWorker {
    #[handler]
    pub async fn identify(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ProfileJob>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let job = input.into_inner();
        validate_key(&ctx, &job)?;
        self.identify_probe(&ctx, &job).await
    }

    #[handler]
    pub async fn gather(
        &self,
        ctx: ObjectContext<'_>,
        input: Json<ProfileJob>,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        let job = input.into_inner();
        validate_key(&ctx, &job)?;
        if let Some(result) = ctx.get::<Json<EvidenceDigest>>("result").await? {
            return Ok(result);
        }
        let artifact = self.build(&ctx, &job).await?;
        publish(&ctx, self.runtime.clone(), artifact).await
    }

    async fn identify_probe(
        &self,
        ctx: &ObjectContext<'_>,
        job: &ProfileJob,
    ) -> Result<Json<EvidenceDigest>, HandlerError> {
        if let Some(probe) = ctx.get::<Json<EvidenceDigest>>("probe").await? {
            return Ok(probe);
        }
        let state = initial_phase(ctx, self.runtime.clone(), job).await?;
        publish_probe(
            ctx,
            self.runtime.clone(),
            profile_probe(job.athlete_id, state),
        )
        .await
    }

    async fn build(
        &self,
        ctx: &ObjectContext<'_>,
        job: &ProfileJob,
    ) -> Result<ProfileAcquisition, HandlerError> {
        let digest = self.identify_probe(ctx, job).await?;
        let state = self
            .runtime
            .load_json::<ProfileProbe>(&digest.0)
            .await
            .map(state_from_probe)
            .map_err(terminal)?;
        team_phase(ctx, self.runtime.clone(), job, state).await
    }
}

fn profile_probe(athlete_id: AthleteId, state: BuildState) -> ProfileProbe {
    ProfileProbe {
        athlete_id,
        responses: state.responses,
        operations: state.operations,
        failures: state.failures,
        profiles: state.profiles,
        identities: state.identities,
        html: state.html,
        requests: state.requests,
        issues: state.issues,
        complete: state.complete,
    }
}

fn state_from_probe(probe: ProfileProbe) -> BuildState {
    BuildState {
        responses: probe.responses,
        operations: probe.operations,
        failures: probe.failures,
        profiles: probe.profiles,
        identities: probe.identities,
        html: probe.html,
        requests: probe.requests,
        issues: probe.issues,
        complete: probe.complete,
    }
}
async fn initial_phase(
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

fn initial_resources(id: AthleteId) -> Result<Vec<SourceResource>, HandlerError> {
    let profile_url = ProfileUrl::parse(&format!(
        "https://www.athletic.net/athlete/{}/track-and-field/all",
        id.get()
    ))
    .map_err(terminal)?;
    Ok(vec![
        SourceResource::Bio {
            athlete_id: id,
            sport: crate::domain::evidence::Sport::TrackField,
        },
        SourceResource::Bio {
            athlete_id: id,
            sport: crate::domain::evidence::Sport::CrossCountry,
        },
        SourceResource::ProfileHtml { profile_url },
    ])
}

async fn acquire(
    ctx: &ObjectContext<'_>,
    snapshot: &EvidenceDigest,
    resources: Vec<SourceResource>,
) -> Result<Vec<(SourceResource, FetchOutcome)>, HandlerError> {
    stream::iter(resources.into_iter().map(Ok::<_, HandlerError>))
        .try_fold(Vec::new(), |mut acquired, resource| async move {
            acquired.push(fetch_one(ctx, snapshot, resource).await?);
            Ok(acquired)
        })
        .await
}

async fn fetch_one(
    ctx: &ObjectContext<'_>,
    snapshot: &EvidenceDigest,
    resource: SourceResource,
) -> Result<(SourceResource, FetchOutcome), HandlerError> {
    let request = SourceRequest {
        snapshot: snapshot.clone(),
        resource: resource.clone(),
    };
    let key = request.key().map_err(terminal)?;
    let outcome = ctx
        .object_client::<SourceCacheClient>(&key)
        .fetch(Json(request))
        .call()
        .await?;
    Ok((resource, outcome.0))
}

async fn parse_sources(
    runtime: Arc<Runtime>,
    athlete: AthleteId,
    fetched: Vec<(SourceResource, FetchOutcome)>,
) -> Vec<ParsedSource> {
    stream::iter(
        fetched
            .into_iter()
            .map(|(resource, outcome)| parse_one(runtime.clone(), athlete, resource, outcome)),
    )
    .buffered(FETCH_CONCURRENCY)
    .collect()
    .await
}

async fn parse_one(
    runtime: Arc<Runtime>,
    athlete: AthleteId,
    resource: SourceResource,
    outcome: FetchOutcome,
) -> ParsedSource {
    let responses = receipts(&outcome);
    let (receipt, failure, source_failure) = match outcome {
        FetchOutcome::Retrieved { receipt, .. } => (Some(receipt), None, None),
        FetchOutcome::Failed { failure } => (None, None, Some(failure)),
    };
    let payload =
        receipt.map(|receipt| parse_document(runtime, athlete, resource.clone(), receipt));
    match payload {
        Some(result) => match result.await {
            Ok(payload) => ParsedSource {
                responses,
                payload: Some(payload),
                failure: None,
                source_failure: None,
            },
            Err(error) => ParsedSource {
                responses,
                payload: None,
                failure: Some(error),
                source_failure: None,
            },
        },
        None => ParsedSource {
            responses,
            payload: None,
            failure,
            source_failure,
        },
    }
}

async fn parse_document(
    runtime: Arc<Runtime>,
    athlete: AthleteId,
    resource: SourceResource,
    receipt: DocumentReceipt,
) -> Result<Payload, String> {
    let store = runtime.store.clone();
    runtime
        .blocking(move || {
            let bytes = store.get_bytes(&receipt.digest)?;
            match resource {
                SourceResource::Bio { sport, .. } => {
                    let root: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_slice(&bytes).context("bio JSON is invalid")?;
                    let (profile, identity) =
                        profile::parse_bio_value(athlete, sport, receipt.digest.clone(), &root)?;
                    let (requests, issues) =
                        team::authorized_requests_value(&root, sport, &receipt.digest)?;
                    Ok(Payload::Bio {
                        profile: Box::new(profile),
                        identity,
                        requests,
                        issues,
                    })
                }
                SourceResource::ProfileHtml { .. } => Ok(Payload::Html(
                    profile::parse_profile_html(athlete, receipt.digest, &bytes)?,
                )),
                SourceResource::Team { .. } | SourceResource::Search { .. } => {
                    Err(anyhow::anyhow!("unexpected resource in profile parser"))
                }
            }
        })
        .await
        .map_err(|error| format!("{error:#}"))
}
fn absorb_initial(mut state: BuildState, item: ParsedSource) -> Result<BuildState, HandlerError> {
    state.responses.extend(item.responses.clone());
    if let Some(source_failure) = item.source_failure {
        state.complete = false;
        state.failures.push(source_failure);
    }
    if let Some(message) = item.failure {
        state.complete = false;
        state.failures.push(failure(
            FailureCode::MalformedResponse,
            message,
            item.responses.clone(),
        )?);
    }
    match item.payload {
        Some(Payload::Bio {
            profile,
            identity,
            requests,
            issues,
        }) => {
            state.profiles.push(*profile);
            if let Some(identity) = identity {
                state.identities.push(identity);
            } else {
                state.complete = false;
            }
            state.requests.extend(requests);
            state.issues.extend(issues);
        }
        Some(Payload::Html(html)) => {
            state.issues.extend(html.issues.clone());
            state.html = Some(html);
        }
        None => state.complete = false,
    }
    Ok(state)
}

async fn team_phase(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    job: &ProfileJob,
    mut state: BuildState,
) -> Result<ProfileAcquisition, HandlerError> {
    let (requests, exceeded) = unique_requests(state.requests);
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

struct ParsedTeam {
    responses: Vec<DocumentReceipt>,
    observation: Option<TeamObservation>,
    failure: Option<String>,
    source_failure: Option<OperationFailure>,
}

async fn parse_teams(
    runtime: Arc<Runtime>,
    fetched: Vec<(SourceResource, FetchOutcome)>,
) -> Vec<ParsedTeam> {
    stream::iter(
        fetched
            .into_iter()
            .map(|(resource, outcome)| parse_team(runtime.clone(), resource, outcome)),
    )
    .buffered(FETCH_CONCURRENCY)
    .collect()
    .await
}

async fn parse_team(
    runtime: Arc<Runtime>,
    resource: SourceResource,
    outcome: FetchOutcome,
) -> ParsedTeam {
    let responses = receipts(&outcome);
    let (receipt, failure, source_failure) = match outcome {
        FetchOutcome::Retrieved { receipt, .. } => (Some(receipt), None, None),
        FetchOutcome::Failed { failure } => (None, None, Some(failure)),
    };
    let Some(receipt) = receipt else {
        return ParsedTeam {
            responses,
            observation: None,
            failure,
            source_failure,
        };
    };
    let SourceResource::Team {
        team_id,
        sport,
        season,
    } = resource
    else {
        return ParsedTeam {
            responses,
            observation: None,
            failure: Some("unexpected non-team resource".to_owned()),
            source_failure: None,
        };
    };
    let store = runtime.store.clone();
    match runtime
        .blocking(move || {
            let bytes = store.get_bytes(&receipt.digest)?;
            team::parse_team_nav(
                TeamRequest {
                    team_id,
                    sport,
                    season,
                },
                receipt.digest,
                &bytes,
            )
        })
        .await
    {
        Ok(observation) => ParsedTeam {
            responses,
            observation: Some(observation),
            failure: None,
            source_failure: None,
        },
        Err(error) => ParsedTeam {
            responses,
            observation: None,
            failure: Some(error.to_string()),
            source_failure: None,
        },
    }
}

fn finalize_profile(
    mut profiles: Vec<ProfileEvidence>,
    html: Option<profile::HtmlProfileEvidence>,
    observations: Vec<TeamObservation>,
    issues: Vec<EvidenceIssue>,
    failures: &mut Vec<OperationFailure>,
    complete: &mut bool,
) -> Result<Option<ProfileEvidence>, HandlerError> {
    if let Some(html) = &html {
        profiles
            .iter_mut()
            .for_each(|profile| profile.profile_url = html.profile_url.clone());
    }
    let merged = profiles
        .drain(..)
        .try_fold(None, |current, next| match current {
            None => Ok(Some(next)),
            Some(left) => profile::merge_profiles(left, next)
                .map(Some)
                .map_err(terminal),
        })?;
    let Some(profile) = merged else {
        if !issues.is_empty() {
            failures.push(failure(
                FailureCode::MalformedResponse,
                "profile parser issues retained without a reconciled profile".to_owned(),
                Vec::new(),
            )?);
        }
        *complete = false;
        return Ok(None);
    };
    let profile = apply_html(profile, html, issues, complete);
    Ok(Some(
        observations
            .into_iter()
            .fold(profile, |profile, observation| {
                attach_team(profile, observation, complete)
            }),
    ))
}

fn apply_html(
    mut profile: ProfileEvidence,
    html: Option<profile::HtmlProfileEvidence>,
    issues: Vec<EvidenceIssue>,
    complete: &mut bool,
) -> ProfileEvidence {
    profile.issues.extend(issues);
    let Some(html) = html else {
        *complete = false;
        return profile;
    };
    if html
        .identity_hints
        .iter()
        .any(|hint| hint.value != profile.name.value)
    {
        *complete = false;
    }
    profile.issues.extend(
        html.identity_hints
            .into_iter()
            .filter(|hint| hint.value != profile.name.value)
            .map(|hint| EvidenceIssue {
                code: "html_identity_inconsistency".to_owned(),
                message: "scoped HTML identity conflicts with bio identity".to_owned(),
                evidence: Some(hint.evidence),
            }),
    );
    html.cohort_witnesses.into_iter().for_each(|witness| {
        if !profile.graduation_years.contains(&witness) {
            profile.graduation_years.push(witness);
        }
    });
    if !profile.documents.contains(&html.document) {
        profile.documents.push(html.document.clone());
    }
    profile.profile_url = html.profile_url;
    if profile
        .issues
        .iter()
        .any(|issue| !issue.is_descriptive_cohort())
    {
        *complete = false;
    }
    profile
}

fn attach_team(
    mut profile: ProfileEvidence,
    observation: TeamObservation,
    complete: &mut bool,
) -> ProfileEvidence {
    if !profile.documents.contains(&observation.evidence.document) {
        profile
            .documents
            .push(observation.evidence.document.clone());
    }
    let joined = profile
        .teams
        .iter()
        .filter(|team| team.team_id == observation.requested.team_id);
    if joined.clone().next().is_none() {
        profile.issues.push(EvidenceIssue {
            code: "team_history_join_missing".to_owned(),
            message: "TeamNav response has no matching bio team history".to_owned(),
            evidence: Some(observation.evidence),
        });
        *complete = false;
        return profile;
    }
    let name_conflict = joined
        .clone()
        .any(|team| team.name.value != observation.name);
    let location_conflict = observation.location.as_ref().is_some_and(|location| {
        joined.clone().any(|team| {
            team.location
                .as_ref()
                .is_some_and(|known| !known.value.compatible_with(location))
        })
    });
    [
        (
            name_conflict,
            "team_name_inconsistency",
            "TeamNav name conflicts with retained team name",
        ),
        (
            location_conflict,
            "team_location_inconsistency",
            "TeamNav location conflicts with retained team location",
        ),
    ]
    .into_iter()
    .filter(|(conflict, _, _)| *conflict)
    .for_each(|(_, code, message)| {
        profile.issues.push(EvidenceIssue {
            code: code.to_owned(),
            message: message.to_owned(),
            evidence: Some(observation.evidence.clone()),
        });
    });
    *complete &= !name_conflict && !location_conflict;
    // Keep independent observations; never invent a location with a single source reference.
    profile.teams.push(TeamEvidence {
        team_id: observation.requested.team_id,
        name: Observed {
            value: observation.name,
            evidence: observation.evidence.clone(),
        },
        location: observation.location.map(|value| Observed {
            value,
            evidence: observation.evidence,
        }),
        seasons: vec![observation.requested.season],
        level: observation.level,
    });
    profile
}

fn unique_requests(requests: Vec<TeamRequest>) -> (Vec<TeamRequest>, bool) {
    let unique = requests
        .into_iter()
        .fold(BTreeMap::new(), |mut map, request| {
            map.entry((request.team_id, request.sport.api_code(), request.season))
                .or_insert(request);
            map
        });
    let exceeded = unique.len() > MAX_TEAM_REQUESTS;
    (
        unique.into_values().take(MAX_TEAM_REQUESTS).collect(),
        exceeded,
    )
}

fn receipts(outcome: &FetchOutcome) -> Vec<DocumentReceipt> {
    match outcome {
        FetchOutcome::Retrieved {
            receipt,
            previous_responses,
            ..
        } => previous_responses
            .iter()
            .cloned()
            .chain(std::iter::once(receipt.clone()))
            .collect(),
        FetchOutcome::Failed { failure } => failure.evidence.clone(),
    }
}

fn retry_evidence(outcome: &FetchOutcome) -> RetryEvidence {
    match outcome {
        FetchOutcome::Retrieved { retries, .. } => retries.clone(),
        FetchOutcome::Failed { failure } => failure.retries.clone(),
    }
}

async fn publish(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    artifact: ProfileAcquisition,
) -> Result<Json<EvidenceDigest>, HandlerError> {
    let digest = ctx
        .run(|| async move { Ok(Json(runtime.store_json(artifact).await.map_err(terminal)?)) })
        .name("profile-acquisition-publication")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    ctx.set("result", Json(digest.0.clone()));
    Ok(digest)
}

async fn publish_probe(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    probe: ProfileProbe,
) -> Result<Json<EvidenceDigest>, HandlerError> {
    let digest = ctx
        .run(|| async move { Ok(Json(runtime.store_json(probe).await.map_err(terminal)?)) })
        .name("profile-identity-probe-publication")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    ctx.set("probe", Json(digest.0.clone()));
    Ok(digest)
}

fn validate_key(ctx: &ObjectContext<'_>, job: &ProfileJob) -> Result<(), HandlerError> {
    if job.key().map_err(terminal)? == ctx.key() {
        Ok(())
    } else {
        Err(terminal(
            "profile worker key does not bind snapshot and athlete",
        ))
    }
}
fn failure(
    code: FailureCode,
    message: String,
    evidence: Vec<DocumentReceipt>,
) -> Result<OperationFailure, HandlerError> {
    Ok(OperationFailure {
        code,
        message,
        http_status: None,
        retries: RetryEvidence::NotAttempted,
        evidence,
    })
}
fn issue(code: &str, message: &str, digest: &EvidenceDigest, locator: &str) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence: Some(EvidenceRef {
            document: digest.clone(),
            locator: locator.to_owned(),
        }),
    }
}
fn terminal(error: impl std::fmt::Display) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}
