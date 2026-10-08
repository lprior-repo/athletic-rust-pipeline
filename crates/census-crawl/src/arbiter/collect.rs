mod run;
mod targets;
mod token;

pub(in crate::arbiter) use run::{Run, Tally, JOURNAL};
pub(in crate::arbiter) use targets::targets;
use token::mint_token;

use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::UsJurisdiction;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub states: Vec<UsJurisdiction>,
    pub observed_on: String,
    pub limit: Option<usize>,
    pub refresh: bool,
}

pub(super) fn fetch_options(
    ctx: &AdapterContext<'_>,
    options: &Options,
    headers: Vec<(String, String)>,
) -> FetchOptions {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = options.refresh || ctx.refresh;
    fetch.headers = headers;
    fetch
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let targets = targets(options)?;
    let org_count = targets.len();
    let stats_before = ctx.fetcher.stats().await;
    let token = mint_token(ctx, options).await?;
    let mut run = Run {
        ctx,
        options,
        fetch: fetch_options(
            ctx,
            options,
            vec![("Authorization".to_string(), format!("Bearer {token}"))],
        ),
        done: ctx.store.journal_keys(JOURNAL)?,
        tally: Tally::default(),
    };
    for (state, org) in targets {
        run.walk(state, org).await?;
    }
    let stats_after = ctx.fetcher.stats().await;
    let mut tally = run.tally;
    let mut report = AdapterReport::new(super::SOURCE_ID, "org_schools");
    report.rows = u64::try_from(tally.schools).map_or(u64::MAX, |value| value);
    report.requests = stats_after
        .physical_requests()
        .saturating_sub(stats_before.physical_requests());
    report.from_cache = stats_after
        .cache_hits
        .saturating_sub(stats_before.cache_hits);
    report.errors = u64::try_from(tally.errors).map_or(u64::MAX, |value| value);
    report.notes = std::mem::take(&mut tally.notes);
    if tally.skipped > 0 {
        report.note(format!(
            "{} school(s) already journaled, skipped",
            tally.skipped
        ));
    }
    report.note(format!(
        "{} school(s) and {} coach row(s) over {org_count} Arbiter organisation(s)",
        tally.schools, tally.coaches
    ));
    Ok(report)
}
