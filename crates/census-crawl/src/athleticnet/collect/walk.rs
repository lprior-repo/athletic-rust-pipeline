use super::{store_accumulated, RunState, PROFILE_ATTEMPT_PHASE, PROFILE_PARSE_VERSION};
use crate::athleticnet::absorb::{absorb, AbsorbContext, AbsorbOutcome};
use crate::athleticnet::{Bio, Options, Scope, Target, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, SCOPES};
use crate::{AdapterContext, CrawlResult, FLUSH_UNITS};
use census_domain::model::{ReviewCase, SourceRef};
use census_domain::school_index::SchoolIndex;
use serde_json::json;
use std::collections::HashSet;

pub(super) fn flush_batch(ctx: &AdapterContext<'_>, run: &mut RunState) -> CrawlResult<()> {
    if run.pending.is_empty() && run.accumulated.profile_reviews.is_empty() {
        return Ok(());
    }
    let mut page = ctx.write_batch();
    let batch = store_accumulated(ctx, std::mem::take(&mut run.accumulated), &mut page)?;
    run.batches.push(batch);
    for (url, payload) in run.pending.drain(..) {
        page.journal_done(PROFILE_ATTEMPT_PHASE, &format!("attempt:{url}"), &payload)?;
        if payload.get("parsed").and_then(serde_json::Value::as_bool) == Some(true) {
            page.journal_done("athleticnet", &url, &payload)?;
        }
    }
    page.commit()?;
    Ok(())
}

pub(super) async fn absorb_targets(
    ctx: &AdapterContext<'_>,
    options: &Options,
    targets: &[Target],
    index: &SchoolIndex,
    done: &HashSet<String>,
    run: &mut RunState,
) -> CrawlResult<()> {
    for (processed, target) in targets.iter().enumerate() {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        run.stats.athletes_seen = run.stats.athletes_seen.saturating_add(1);
        let mut absorbed_any = false;
        for scope in SCOPES {
            let absorbed = absorb_scope(ctx, options, target, scope, (index, done), run).await?;
            absorbed_any |= absorbed;
        }
        if absorbed_any {
            run.stats.athletes_absorbed = run.stats.athletes_absorbed.saturating_add(1);
        }
    }
    Ok(())
}

async fn absorb_scope(
    ctx: &AdapterContext<'_>,
    options: &Options,
    target: &Target,
    scope: Scope,
    retained: (&SchoolIndex, &HashSet<String>),
    run: &mut RunState,
) -> CrawlResult<bool> {
    let url = format!(
        "{BIO_ENDPOINT}?athleteId={}&sport={}&level={HIGH_SCHOOL_LEVEL}",
        target.athlete_id,
        scope.parameter()
    );
    if !options.refresh && retained.1.contains(&url) {
        return Ok(false);
    }
    let Some((bio, source)) = fetch_bio(ctx, target, scope, options, &url, run).await else {
        flush_batch(ctx, run)?;
        return Ok(false);
    };
    let outcome = match absorb(
        &bio,
        scope,
        target,
        AbsorbContext {
            source: &source,
            observed_on: &options.observed_on,
            index: retained.0,
            resolved: &mut run.resolved,
            stats: &mut run.stats,
            accumulated: &mut run.accumulated,
        },
    ) {
        Ok(outcome) => outcome,
        Err(error) => {
            stage_failure(run, &url, &error.to_string());
            flush_batch(ctx, run)?;
            return Ok(false);
        }
    };
    stage_outcome(run, target, scope, url, &outcome);
    if run.pending.len() >= FLUSH_UNITS {
        flush_batch(ctx, run)?;
    }
    Ok(matches!(
        outcome,
        AbsorbOutcome::Complete { .. } | AbsorbOutcome::Partial { .. }
    ))
}

fn stage_outcome(
    run: &mut RunState,
    target: &Target,
    scope: Scope,
    url: String,
    outcome: &AbsorbOutcome,
) {
    let complete = matches!(outcome, AbsorbOutcome::Complete { .. });
    if !complete {
        run.stats.fetches_failed = run.stats.fetches_failed.saturating_add(1);
        run.report
            .note(format!("athlete {}: {outcome:?}", target.athlete_id));
    }
    run.pending.push((
        url.clone(),
        json!({
            "url": url,
            "parser": PROFILE_PARSE_VERSION,
            "parsed": complete,
            "athlete": target.athlete_id,
            "sport": scope.parameter(),
            "rows": outcome.rows(),
            "outcome": format!("{outcome:?}"),
        }),
    ));
}

async fn fetch_bio(
    ctx: &AdapterContext<'_>,
    target: &Target,
    scope: Scope,
    options: &Options,
    url: &str,
    run: &mut RunState,
) -> Option<(Bio, SourceRef)> {
    let mut fetch_options = ctx.fetch_options();
    fetch_options.refresh = options.refresh;
    fetch_options.headers = vec![("Accept".to_string(), "application/json".to_string())];
    let fetched = match ctx.fetcher.get(url, &fetch_options).await {
        Ok(fetched) => fetched,
        Err(error) => {
            record_failure(run, url, &format!("athlete {}: {error}", target.athlete_id));
            return None;
        }
    };
    match serde_json::from_str(&fetched.text()) {
        Ok(bio) => Some((bio, SourceRef::new("athleticnet", Some(fetched.url)))),
        Err(error) => {
            record_failure(
                run,
                url,
                &format!(
                    "athlete {} {}: body is not an athlete bio ({error})",
                    target.athlete_id,
                    scope.parameter()
                ),
            );
            None
        }
    }
}

fn record_failure(run: &mut RunState, url: &str, detail: &str) {
    run.accumulated.profile_reviews.push(ReviewCase::pending(
        "athleticnet_profile_rejection",
        url,
        url,
        format!("{detail}. Source: {url}"),
    ));
    stage_failure(run, url, detail);
}

fn stage_failure(run: &mut RunState, url: &str, detail: &str) {
    run.stats.fetches_failed = run.stats.fetches_failed.saturating_add(1);
    run.report.note(detail.to_string());
    run.pending.push((
        url.to_string(),
        json!({
            "url": url,
            "parser": PROFILE_PARSE_VERSION,
            "parsed": false,
            "error": detail,
        }),
    ));
}
