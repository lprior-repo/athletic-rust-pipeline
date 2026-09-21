use super::super::{
    protocol::{DocumentReceipt, FetchOutcome, RetryEvidence, SourceResource},
    source_cache::{SourceCacheClient, SourceRequest},
};
use crate::domain::identity::{AthleteId, EvidenceDigest, ProfileUrl};
use anyhow::Result;
use futures::{stream, TryStreamExt};
use restate_sdk::prelude::*;

use super::reporting::terminal;

pub(super) fn initial_resources(id: AthleteId) -> Result<Vec<SourceResource>, HandlerError> {
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

pub(super) async fn acquire(
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

pub(super) fn receipts(outcome: &FetchOutcome) -> Vec<DocumentReceipt> {
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

pub(super) fn retry_evidence(outcome: &FetchOutcome) -> RetryEvidence {
    match outcome {
        FetchOutcome::Retrieved { retries, .. } => retries.clone(),
        FetchOutcome::Failed { failure } => failure.retries.clone(),
    }
}
