use census_crawl::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::UsJurisdiction;

pub(super) async fn ohsaa(
    ctx: &AdapterContext<'_>,
    state: UsJurisdiction,
) -> CrawlResult<AdapterReport> {
    census_crawl::ohsaa::collect(
        ctx,
        &census_crawl::ohsaa::Options {
            limit: None,
            refresh: ctx.refresh,
            observed_on: ctx.observed_on.clone(),
            states: vec![state],
            school_names: Vec::new(),
        },
    )
    .await
}

pub(super) async fn mpa(
    ctx: &AdapterContext<'_>,
    state: UsJurisdiction,
) -> CrawlResult<AdapterReport> {
    census_crawl::mpa::collect(
        ctx,
        &census_crawl::mpa::Options {
            limit: None,
            refresh: ctx.refresh,
            observed_on: ctx.observed_on.clone(),
            states: vec![state],
            school_names: Vec::new(),
        },
    )
    .await
}

pub(super) async fn riil(ctx: &AdapterContext<'_>) -> CrawlResult<AdapterReport> {
    census_crawl::riil::collect(
        ctx,
        &census_crawl::riil::Options {
            limit: None,
            refresh: ctx.refresh,
            observed_on: ctx.observed_on.clone(),
        },
    )
    .await
}

pub(super) async fn piaa(
    ctx: &AdapterContext<'_>,
    state: UsJurisdiction,
) -> CrawlResult<AdapterReport> {
    census_crawl::pa_piaa::collect(
        ctx,
        &census_crawl::pa_piaa::Options {
            limit: None,
            refresh: ctx.refresh,
            observed_on: ctx.observed_on.clone(),
            states: vec![state],
            letters: Vec::new(),
            details_names: Vec::new(),
        },
    )
    .await
}

pub(super) async fn chsaa(
    ctx: &AdapterContext<'_>,
    state: UsJurisdiction,
) -> CrawlResult<AdapterReport> {
    census_crawl::chsaa::collect(
        ctx,
        &census_crawl::chsaa::Options {
            limit: None,
            refresh: ctx.refresh,
            observed_on: ctx.observed_on.clone(),
            states: vec![state],
            school_names: Vec::new(),
        },
    )
    .await
}

pub(super) async fn tssaa(
    ctx: &AdapterContext<'_>,
    state: UsJurisdiction,
) -> CrawlResult<AdapterReport> {
    census_crawl::tssaa::collect(
        ctx,
        &census_crawl::tssaa::Options {
            limit: None,
            refresh: ctx.refresh,
            observed_on: ctx.observed_on.clone(),
            states: vec![state],
            school_names: Vec::new(),
        },
    )
    .await
}
