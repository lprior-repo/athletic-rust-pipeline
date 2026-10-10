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
use census_store::{StoreError, Table};
use futures::{stream, StreamExt, TryStreamExt};

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
        report
            .unfinished
            .push("tfrrs:configured-page-urls".to_string());
        report.note("no page URLs supplied; acquisition remains unqualified");
        return Ok(report);
    }
    let (requests_before, cache_before) = stats_of(ctx).await;
    let schools = consolidated_schools(ctx)?;
    let index = SchoolIndex::from_schools(&schools);
    let run = Run::new(&index);
    let mut run = stream::iter(options.urls.iter().enumerate())
        .map(Ok::<_, CrawlError>)
        .try_fold(run, |mut run, (index, url)| async move {
            if index >= limit_of(options) {
                run.unfinished
                    .try_reserve(1)
                    .map_err(|_| CrawlError::Resource {
                        resource: "TFRRS held frontier",
                        requested: 1,
                        limit: options.urls.len(),
                    })?;
                run.unfinished.push(url.clone());
            } else {
                run.read(ctx, url).await;
            }
            Ok(run)
        })
        .await?;
    let counts = run.append(ctx, &schools)?;
    finish(
        report,
        run,
        counts,
        (requests_before, cache_before),
        stats_of(ctx).await,
    )
}

fn finish(
    mut report: AdapterReport,
    run: Run<'_>,
    counts: report::EntityCounts,
    before: (u64, u64),
    after: (u64, u64),
) -> CrawlResult<AdapterReport> {
    report.rows = run
        .absorb
        .stats
        .rows_absorbed
        .checked_add(run.absorb.stats.roster_rows_absorbed)
        .ok_or_else(counter_error)?;
    report.requests = after.0.checked_sub(before.0).ok_or_else(counter_error)?;
    report.from_cache = after.1.checked_sub(before.1).ok_or_else(counter_error)?;
    report.errors = u64::try_from(run.failures.len()).map_err(|_| counter_error())?;
    report::note_pages(&mut report, &run);
    report::note_rows(&mut report, &run.absorb.stats);
    report::note_rosters(&mut report, &run.absorb.stats);
    report::note_resolution(&mut report, &run.absorb.stats);
    report::note_entities(&mut report, &counts);
    let failures = run
        .failures
        .into_iter()
        .take(5)
        .map(|failure| failure.chars().take(4096).collect())
        .collect::<Vec<String>>();
    report::note_failures(&mut report, &failures);
    report.unfinished = run.unfinished;
    report.finish_frontier();
    Ok(report)
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "TFRRS acquisition accounting overflow".to_string(),
    }
}

fn limit_of(options: &Options) -> usize {
    options.limit.map_or(usize::MAX, |value| value)
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.physical_requests(), stats.cache_hits)
}

fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    ctx.store
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            schools.push(school);
            if schools.len() > crate::SCHOOL_BINDINGS {
                return Err(StoreError::Invariant {
                    detail: "result school-binding resource limit; projection remains unfinished"
                        .into(),
                });
            }
            Ok(())
        })?;
    Ok(schools)
}
