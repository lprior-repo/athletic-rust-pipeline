use crate::{CrawlError, CrawlResult};

mod map;
mod parse;
mod report;
mod run;

#[cfg(test)]
mod tests;

pub use parse::{
    parse_list_page, parse_list_path, parse_team_page, parse_team_path, ListPath, TeamPath,
};

use crate::{AdapterContext, AdapterReport};
use census_domain::model::CanonicalSchool;
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

use run::Run;

const ADAPTER: &str = "tfrrs";

const PHASE: &str = "tfrrs_pages_v2";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub urls: Vec<String>,
    pub limit: Option<usize>,
    pub observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    List(ListPath),
    Team(TeamPath),
}

pub fn classify(url: &str) -> Option<Route> {
    parse_list_path(url)
        .map(Route::List)
        .or_else(|| parse_team_path(url).map(Route::Team))
}

fn source_id(state: UsJurisdiction) -> String {
    format!("tfrrs_{}", state.code().to_ascii_lowercase())
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER, "rows");
    if options.urls.is_empty() {
        report.note("no page URLs supplied; nothing was requested".to_string());
        return Ok(report);
    }
    let (requests_before, cache_before) = stats_of(ctx).await;
    let schools = consolidated_schools(ctx)?;
    let index = SchoolIndex::from_schools(&schools);
    let mut run = Run::new(&index, ctx.store.journal_keys(PHASE)?);
    for url in options.urls.iter().take(limit_of(options)) {
        run.read(ctx, url).await;
    }
    let counts = run.append(ctx, &schools)?;
    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = run
        .absorb
        .stats
        .rows_absorbed
        .saturating_add(run.absorb.stats.roster_rows_absorbed);
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.errors = u64::try_from(run.failures.len()).map_err(|_| CrawlError::Arithmetic {
        detail: "failure count does not fit in u64".to_string(),
    })?;
    report::note_pages(&mut report, &run);
    report::note_rows(&mut report, &run.absorb.stats);
    report::note_rosters(&mut report, &run.absorb.stats);
    report::note_resolution(&mut report, &run.absorb.stats);
    report::note_entities(&mut report, &counts);
    report::note_failures(&mut report, &run.failures);
    Ok(report)
}

fn limit_of(options: &Options) -> usize {
    options.limit.map_or(usize::MAX, |value| value)
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let path = ctx.store.out_dir().join("schools.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let schools: Vec<CanonicalSchool> = census_store::read::read_rows(&path)?;
    Ok(schools)
}
