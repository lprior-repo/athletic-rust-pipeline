mod run;
mod teams;

use super::Options;
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};

fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        ..ctx.fetch_options()
    }
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut run = run::MshslRun::start(ctx, options);
    run.walk().await?;
    if run.report.unfinished.is_empty() && run.selected {
        run.report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    run.report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(run.report)
}
