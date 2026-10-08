mod run;
mod targets;
mod token;

pub(in crate::arbiter) use run::{Run, Tally};
pub(in crate::arbiter) use targets::targets;
use token::mint_token;

use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};

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
    let mut report = AdapterReport::new(super::SOURCE_ID, "org_schools");
    if targets.is_empty() {
        crate::directory::acquisition::owe(&mut report, super::DIRECTORY_URL)?;
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    let token = match mint_token(ctx, options).await {
        Ok(token) => token,
        Err(CrawlError::Store(error)) => return Err(CrawlError::Store(error)),
        Err(error) => {
            crate::directory::acquisition::fail(&mut report, super::DIRECTORY_URL, error)?;
            crate::directory::acquisition::owe(&mut report, super::TOKEN_URL)?;
            return Ok(report);
        }
    };
    let run = Run {
        ctx,
        options,
        fetch: fetch_options(
            ctx,
            options,
            vec![("Authorization".to_owned(), format!("Bearer {token}"))],
        ),
        tally: Tally::default(),
    };
    let run = stream::iter(targets)
        .map(Ok::<_, CrawlError>)
        .try_fold(run, |mut run, (state, org)| async move {
            run.walk(state, org).await?;
            Ok(run)
        })
        .await?;
    let after = ctx.fetcher.stats().await;
    report.rows = u64::try_from(run.tally.schools).map_or(u64::MAX, |value| value);
    report.errors = u64::try_from(run.tally.errors).map_or(u64::MAX, |value| value);
    report.note(format!(
        "{} school(s) and {} coach row(s); {} school(s) already journaled",
        run.tally.schools, run.tally.coaches, run.tally.skipped,
    ));
    report.notes.extend(run.tally.notes);
    report.unfinished = run.tally.unfinished;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    if report.unfinished.is_empty() {
        report.finish_frontier();
    } else {
        report.disposition = crate::CollectionDisposition::Partial;
    }
    Ok(report)
}
