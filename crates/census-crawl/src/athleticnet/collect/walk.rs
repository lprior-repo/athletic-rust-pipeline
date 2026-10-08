use super::{EntityCounts, RunState, PROFILE_ATTEMPT_PHASE, PROFILE_PARSE_VERSION};
use crate::athleticnet::absorb::{absorb, AbsorbContext, AbsorbOutcome};
use crate::athleticnet::{Options, Scope, Target, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, SCOPES};
use crate::net::{FetchError, FetchOutcome};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{ReviewCase, SourceRef};
use census_domain::school_index::SchoolIndex;
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::json;

pub(super) fn flush_batch(ctx: &AdapterContext<'_>, run: &mut RunState) -> CrawlResult<()> {
    if run.pending.is_empty() && run.accumulated.profile_reviews.is_empty() {
        return Ok(());
    }
    let counts = super::effects::persist(ctx, std::mem::take(&mut run.accumulated))?;
    let mut page = ctx.write_batch();
    run.pending
        .drain(..)
        .try_for_each(|(key, payload)| page.journal_done(PROFILE_ATTEMPT_PHASE, &key, &payload))?;
    page.commit()?;
    total_counts(&mut run.batches, counts)?;
    run.resolved.clear();
    Ok(())
}

fn total_counts(batches: &mut Vec<EntityCounts>, counts: EntityCounts) -> CrawlResult<()> {
    let Some(total) = batches.first_mut() else {
        batches
            .try_reserve(1)
            .map_err(|_| resource("NET counts", 1))?;
        batches.push(counts);
        return Ok(());
    };
    total.schools = total
        .schools
        .checked_add(counts.schools)
        .ok_or_else(counter_error)?;
    total.meets = total
        .meets
        .checked_add(counts.meets)
        .ok_or_else(counter_error)?;
    total.teams = total
        .teams
        .checked_add(counts.teams)
        .ok_or_else(counter_error)?;
    total.athletes = total
        .athletes
        .checked_add(counts.athletes)
        .ok_or_else(counter_error)?;
    total.events = total
        .events
        .checked_add(counts.events)
        .ok_or_else(counter_error)?;
    total.performances = total
        .performances
        .checked_add(counts.performances)
        .ok_or_else(counter_error)?;
    total.unsupported_cohorts = total
        .unsupported_cohorts
        .checked_add(counts.unsupported_cohorts)
        .ok_or_else(counter_error)?;
    Ok(())
}

pub(super) async fn absorb_targets(
    ctx: &AdapterContext<'_>,
    options: &Options,
    targets: &[Target],
    index: &SchoolIndex,
    run: &mut RunState,
) -> CrawlResult<()> {
    stream::iter(targets.iter().enumerate())
        .map(Ok::<_, CrawlError>)
        .try_fold(run, |run, (processed, target)| async move {
            if options.limit.is_some_and(|limit| processed >= limit) {
                SCOPES
                    .iter()
                    .try_for_each(|scope| owe(run, &profile_request(target, *scope)))?;
            } else {
                absorb_target(ctx, options, target, index, run).await?;
            }
            Ok(run)
        })
        .await?;
    Ok(())
}

#[tracing::instrument(skip(ctx, options, target, index, run))]
async fn absorb_target(
    ctx: &AdapterContext<'_>,
    options: &Options,
    target: &Target,
    index: &SchoolIndex,
    run: &mut RunState,
) -> CrawlResult<()> {
    run.stats.athletes_seen = run
        .stats
        .athletes_seen
        .checked_add(1)
        .ok_or_else(counter_error)?;
    let (run, absorbed) = stream::iter(SCOPES)
        .map(Ok::<_, CrawlError>)
        .try_fold((run, false), |(run, any), scope| async move {
            let absorbed = absorb_scope(ctx, options, (target, scope), index, run).await?;
            Ok((run, any || absorbed))
        })
        .await?;
    if absorbed {
        run.stats.athletes_absorbed = run
            .stats
            .athletes_absorbed
            .checked_add(1)
            .ok_or_else(counter_error)?;
    }
    Ok(())
}

#[tracing::instrument(skip(ctx, options, target_scope, index, run))]
async fn absorb_scope(
    ctx: &AdapterContext<'_>,
    options: &Options,
    target_scope: (&Target, Scope),
    index: &SchoolIndex,
    run: &mut RunState,
) -> CrawlResult<bool> {
    let (target, scope) = target_scope;
    let url = profile_request(target, scope);
    let capture = match fetch(ctx, options, &url).await {
        Ok(capture) => capture,
        Err(error) => {
            record_failure(run, &url, &error.to_string())?;
            flush_batch(ctx, run)?;
            return Ok(false);
        }
    };
    let physical = AdapterContext {
        observed_on: capture.fetched_at.clone(),
        ..*ctx
    };
    if let Err(error) = super::admission::check(&capture.body) {
        record_failure(run, &url, &error.to_string())?;
        flush_batch(&physical, run)?;
        return Ok(false);
    }
    let bio = match serde_json::from_slice(&capture.body) {
        Ok(bio) => bio,
        Err(error) => {
            record_failure(run, &url, &error.to_string())?;
            flush_batch(&physical, run)?;
            return Ok(false);
        }
    };
    let source = SourceRef::new("athleticnet", Some(capture.url.clone()));
    let outcome = absorb(
        &bio,
        scope,
        target,
        AbsorbContext {
            source: &source,
            observed_on: &capture.fetched_at,
            performance_as_of: ctx.performance_as_of,
            index,
            resolved: &mut run.resolved,
            stats: &mut run.stats,
            accumulated: &mut run.accumulated,
        },
    );
    complete_scope(&physical, run, &capture, outcome)
}

fn complete_scope(
    ctx: &AdapterContext<'_>,
    run: &mut RunState,
    capture: &FetchOutcome,
    outcome: CrawlResult<AbsorbOutcome>,
) -> CrawlResult<bool> {
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            record_failure(run, &capture.url, &error.to_string())?;
            flush_batch(ctx, run)?;
            return Ok(false);
        }
    };
    let complete = matches!(outcome, AbsorbOutcome::Complete { .. });
    if !complete {
        run.stats.fetches_failed = run
            .stats
            .fetches_failed
            .checked_add(1)
            .ok_or_else(counter_error)?;
        owe(run, &capture.url)?;
    }
    run.pending
        .try_reserve(1)
        .map_err(|_| resource("NET pending scope", 1))?;
    run.pending.push((capture.url.clone(), json!({
        "url":capture.url,"capture_sha256":capture.content_digest,"fetched_at":capture.fetched_at,
        "parser":PROFILE_PARSE_VERSION,"parsed":complete,"rows":outcome.rows(),"outcome":format!("{outcome:?}"),
        "school_year":ctx.school_year,"performance_as_of":ctx.performance_as_of,
    })));
    flush_batch(ctx, run)?;
    Ok(matches!(
        outcome,
        AbsorbOutcome::Complete { .. } | AbsorbOutcome::Partial { .. }
    ))
}

#[tracing::instrument(skip(ctx, options))]
async fn fetch(
    ctx: &AdapterContext<'_>,
    options: &Options,
    url: &str,
) -> Result<FetchOutcome, FetchError> {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = ctx.refresh || options.refresh;
    fetch.headers = vec![("Accept".to_string(), "application/json".to_string())];
    ctx.fetcher.get(url, &fetch).await
}

fn profile_request(target: &Target, scope: Scope) -> String {
    format!(
        "{BIO_ENDPOINT}?athleteId={}&sport={}&level={HIGH_SCHOOL_LEVEL}",
        target.athlete_id,
        scope.parameter()
    )
}

fn record_failure(run: &mut RunState, url: &str, detail: &str) -> CrawlResult<()> {
    let detail: String = detail.chars().take(4096).collect();
    run.accumulated
        .profile_reviews
        .try_reserve(1)
        .map_err(|_| resource("NET rejection", 1))?;
    run.accumulated.profile_reviews.push(ReviewCase::pending(
        "athleticnet_profile_rejection",
        url,
        url,
        detail.clone(),
    ));
    run.stats.fetches_failed = run
        .stats
        .fetches_failed
        .checked_add(1)
        .ok_or_else(counter_error)?;
    owe(run, url)?;
    if run.stats.fetches_failed <= 5 {
        run.report.note(detail.clone());
    }
    run.pending
        .try_reserve(1)
        .map_err(|_| resource("NET rejected scope", 1))?;
    run.pending.push((
        url.to_string(),
        json!({"url":url,"parser":PROFILE_PARSE_VERSION,"parsed":false,"error":detail}),
    ));
    Ok(())
}

fn owe(run: &mut RunState, url: &str) -> CrawlResult<()> {
    if run.report.unfinished.iter().any(|value| value == url) {
        return Ok(());
    }
    run.report
        .unfinished
        .try_reserve(1)
        .map_err(|_| resource("NET unfinished scope", 1))?;
    run.report.unfinished.push(url.to_string());
    Ok(())
}

pub(super) fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "NET counter overflow".to_string(),
    }
}

fn resource(resource: &'static str, requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit: crate::net::MAX_BODY_BYTES,
    }
}
