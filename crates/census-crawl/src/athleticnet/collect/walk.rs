//! The bio walk: one request per (athlete, sport) pair, absorbed into the run's accumulator, then
//! appended and journaled per unit so a re-run resumes without re-reading what is already stored.
//!
//! Split out of the parent module when that file passed the repository's file budget; the dispatch,
//! the resume reads and the accumulator append stay there.

use super::{store_accumulated, RunState};
use crate::athleticnet::absorb::absorb;
use crate::athleticnet::parse::Bio;
use crate::athleticnet::{
    Options, Scope, Target, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, PARSE_VERSION, SCOPES,
};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::SourceRef;
use census_domain::school_index::SchoolIndex;
use serde_json::json;
use std::collections::HashSet;

/// Units one flush covers: a batch's rows are appended, then its units are journaled.
const FLUSH_UNITS: usize = 64;

/// Append the batch's rows, then journal its units: `append_many` commits at `SyncData` and a
/// journal write commits separately, so a journal entry may only claim rows already appended.
pub(super) fn flush_batch(ctx: &AdapterContext<'_>, run: &mut RunState) -> CrawlResult<()> {
    if run.pending.is_empty() {
        return Ok(());
    }
    let batch = store_accumulated(ctx, std::mem::take(&mut run.accumulated))?;
    run.batches.push(batch);
    for (url, payload) in run.pending.drain(..) {
        ctx.store.journal_done("athleticnet", &url, &payload)?;
    }
    Ok(())
}

/// Absorb every pending (athlete, sport) payload into the run's accumulation.
pub(super) async fn absorb_targets(
    ctx: &AdapterContext<'_>,
    options: &Options,
    targets: &[Target],
    source: &SourceRef,
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
            let url = format!(
                "{BIO_ENDPOINT}?athleteId={}&sport={}&level={HIGH_SCHOOL_LEVEL}",
                target.athlete_id,
                scope.parameter()
            );
            if done.contains(&url) {
                continue;
            }
            let Some(bio) = fetch_bio(ctx, target, scope, options, &url, run).await else {
                continue;
            };
            let rows = absorb(
                &bio,
                scope,
                target,
                source,
                &options.observed_on,
                index,
                &mut run.resolved,
                &mut run.stats,
                &mut run.accumulated,
            );
            absorbed_any |= rows > 0;
            let payload = json!({
                "url": url,
                "parser": PARSE_VERSION,
                "parsed": true,
                "athlete": target.athlete_id,
                "sport": scope.parameter(),
                "rows": rows,
            });
            run.pending.push((url, payload));
            if run.pending.len() >= FLUSH_UNITS {
                flush_batch(ctx, run)?;
            }
        }
        if absorbed_any {
            run.stats.athletes_absorbed = run.stats.athletes_absorbed.saturating_add(1);
        }
    }
    Ok(())
}

/// Fetch and decode one (athlete, sport) payload, or `None` when it could not be read.
async fn fetch_bio(
    ctx: &AdapterContext<'_>,
    target: &Target,
    scope: Scope,
    options: &Options,
    url: &str,
    run: &mut RunState,
) -> Option<Bio> {
    let mut fetch_options = ctx.fetch_options();
    fetch_options.refresh = options.refresh;
    fetch_options.headers = vec![("Accept".to_string(), "application/json".to_string())];
    let fetched = match ctx.fetcher.get(url, &fetch_options).await {
        Ok(fetched) => fetched,
        Err(error) => {
            run.stats.fetches_failed = run.stats.fetches_failed.saturating_add(1);
            run.report
                .note(format!("athlete {}: {error}", target.athlete_id));
            return None;
        }
    };
    match serde_json::from_str(&fetched.text()) {
        Ok(bio) => Some(bio),
        Err(error) => {
            run.stats.fetches_failed = run.stats.fetches_failed.saturating_add(1);
            run.report.note(format!(
                "athlete {} {}: body is not an athlete bio ({error})",
                target.athlete_id,
                scope.parameter()
            ));
            None
        }
    }
}
