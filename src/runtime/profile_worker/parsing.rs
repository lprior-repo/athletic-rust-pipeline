use super::super::{
    acquisition::TeamRequest,
    protocol::{DocumentReceipt, FetchOutcome, OperationFailure, SourceResource},
    Runtime,
};
use super::intake::receipts;
use super::team::{self, TeamObservation};
use crate::{
    domain::{
        evidence::{EvidenceIssue, ProfileEvidence},
        identity::AthleteId,
        name::BioIdentityObservation,
    },
    profile,
};
use anyhow::{Context, Result};
use futures::{stream, StreamExt};
use std::sync::Arc;

const FETCH_CONCURRENCY: usize = 8;

#[derive(Debug)]
pub(super) enum Payload {
    Bio {
        profile: Box<ProfileEvidence>,
        identity: Option<BioIdentityObservation>,
        requests: Vec<TeamRequest>,
        issues: Vec<EvidenceIssue>,
    },
    Html(profile::HtmlProfileEvidence),
}

pub(super) struct ParsedSource {
    pub(super) responses: Vec<DocumentReceipt>,
    pub(super) payload: Option<Payload>,
    pub(super) failure: Option<String>,
    pub(super) source_failure: Option<OperationFailure>,
}

pub(super) struct ParsedTeam {
    pub(super) responses: Vec<DocumentReceipt>,
    pub(super) observation: Option<TeamObservation>,
    pub(super) failure: Option<String>,
    pub(super) source_failure: Option<OperationFailure>,
}

pub(super) async fn parse_sources(
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
                SourceResource::Rankings { .. } => Err(anyhow::anyhow!(
                    "rankings resource not supported in profile parser"
                )),
                SourceResource::Search { .. } | SourceResource::Team { .. } => {
                    Err(anyhow::anyhow!("unexpected resource in profile parser"))
                }
            }
        })
        .await
        .map_err(|error| format!("{error:#}"))
}

pub(super) async fn parse_teams(
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
