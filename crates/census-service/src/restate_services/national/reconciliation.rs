use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::review_model::{ModelOptions, ModelResponseFormat};
use census_reconcile::identity::WorkflowIdentity;
use census_review::{ModelClient, ReviewFamily, ReviewOptions};
use census_store::MAX_ROWS_PER_TABLE;
use restate_sdk::prelude::*;

use super::NationalCensus;
use crate::restate_services::{blocking, job_error, JobError};

pub(super) fn model_lanes() -> Result<[ModelClient; 2], String> {
    let build = |endpoint, format| {
        ModelOptions::local(endpoint, "qwen3.8-27b-uncensored")
            .and_then(|options| ModelClient::new(options.with_response_format(format)))
            .map_err(|error| error.to_string())
    };
    Ok([
        build("http://127.0.0.1:11000", ModelResponseFormat::PromptJson)?,
        build("http://127.0.0.1:11001", ModelResponseFormat::JsonSchema)?,
    ])
}

#[tracing::instrument(skip_all, fields(run = identity.as_str()))]
pub(super) async fn run(
    national: &NationalCensus,
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    season: SchoolYear,
    observed_at: &str,
) -> Result<(), HandlerError> {
    rules(national, ctx, identity, season, observed_at).await?;
    advice(national, ctx, observed_at).await?;
    derive(national, ctx, identity, season, observed_at, "reconcile").await
}

#[tracing::instrument(skip_all)]
async fn derive(
    national: &NationalCensus,
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    season: SchoolYear,
    observed_at: &str,
    stage: &str,
) -> Result<(), HandlerError> {
    let jobs = national.jobs.clone();
    let phase = format!("{}:identity:{stage}", identity.as_str());
    let at = observed_at.to_owned();
    let name = format!("identity:{stage}");
    let reply = ctx
        .run(move || async move {
            let permit = jobs.permit_control().await?;
            let store = Arc::clone(jobs.store());
            blocking(Arc::clone(jobs.region()), move || {
                let _permit = permit;
                census_reconcile::index::derive(&store, &phase, &at, season).map(|report| {
                    Json(format!(
                        "identities={} conflicts={} reviews={} applied={}",
                        report.source_identities,
                        report.conflicts,
                        report.reviews,
                        report.identity_applications
                    ))
                })
            })
            .await
            .map_err(job_error)
        })
        .name(name.clone())
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    ctx.set(&name, reply);
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn rules(
    national: &NationalCensus,
    ctx: &WorkflowContext<'_>,
    identity: &WorkflowIdentity,
    season: SchoolYear,
    observed_at: &str,
) -> Result<(), HandlerError> {
    let jobs = national.jobs.clone();
    let at = observed_at.to_owned();
    let phase = format!("{}:identity:rules", identity.as_str());
    let reply = ctx
        .run(move || async move {
            let permit = jobs.permit_control().await?;
            let store = Arc::clone(jobs.store());
            blocking(Arc::clone(jobs.region()), move || {
                let _permit = permit;
                let report = census_review::reconcile_athletes(&store, &at, false)
                    .map_err(JobError::from)?;
                census_reconcile::index::derive(&store, &phase, &at, season)
                    .map_err(JobError::from)?;
                Ok::<_, JobError>(Json(report.summary()))
            })
            .await
            .map_err(job_error)
        })
        .name("identity:rules")
        .retry_policy(RunRetryPolicy::new().max_attempts(1))
        .await?;
    ctx.set("identity:rules", reply);
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn advice(
    national: &NationalCensus,
    ctx: &WorkflowContext<'_>,
    observed_at: &str,
) -> Result<(), HandlerError> {
    let mut after = None;
    let budget = MAX_ROWS_PER_TABLE
        .checked_mul(2)
        .ok_or_else(|| TerminalError::new("review checkpoint budget overflow"))?;
    for checkpoint in 0..=budget {
        let reply =
            checkpoint_advice(national, ctx, observed_at, after.clone(), checkpoint).await?;
        let next = reply.0 .1.clone();
        ctx.set("identity:advice", reply);
        match next {
            None => return Ok(()),
            Some(cursor) if after.as_ref().is_none_or(|previous| &cursor > previous) => {
                after = Some(cursor)
            }
            Some(_) => {
                return Err(TerminalError::new("review checkpoint cursor did not advance").into())
            }
        }
    }
    Err(TerminalError::new("review checkpoint count exceeds the bounded case inventory").into())
}

#[tracing::instrument(skip_all)]
async fn checkpoint_advice(
    national: &NationalCensus,
    ctx: &WorkflowContext<'_>,
    observed_at: &str,
    after: Option<String>,
    checkpoint: u64,
) -> Result<Json<(String, Option<String>)>, HandlerError> {
    let jobs = national.jobs.clone();
    let lanes = national.review_lanes.clone();
    let at = observed_at.to_owned();
    let runtime = tokio::runtime::Handle::current();
    ctx.run(move || async move {
        let permit = jobs.permit_control().await?;
        let store = Arc::clone(jobs.store());
        blocking(Arc::clone(jobs.region()), move || {
            let _permit = permit;
            let clients = lanes.map_err(|message| JobError::Terminal { message })?;
            let options = ReviewOptions {
                families: ReviewFamily::askable().to_vec(),
                limit: 256,
                dry_run: false,
            };
            runtime
                .block_on(census_review::run_lanes_after(
                    &store,
                    &clients,
                    &options,
                    &at,
                    after.as_deref(),
                ))
                .map(|(report, cursor)| Json((report.summary(), cursor)))
                .map_err(JobError::from)
        })
        .await
        .map_err(job_error)
    })
    .name(format!("identity:advice:{checkpoint}"))
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map_err(HandlerError::from)
}
