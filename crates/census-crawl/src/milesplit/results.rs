use super::wire::ResultSetRef;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::Table;
use std::collections::HashMap;

mod pages;
mod report;
mod run;
#[cfg(test)]
mod tests;

pub use pages::{
    is_results_page, read_meet_pages, ListedResultFile, MeetPage, MeetPages, MISMATCH_LIMIT,
};
use report::{note_entities, note_resolution, note_result_sets, note_rows};
use run::Run;

const PHASE: &str = "milesplit_result_sets_v2";
const ADAPTER: &str = "milesplit_results";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetRequest {
    pub url: String,
    pub jurisdiction: UsJurisdiction,
}

#[derive(Debug, Clone, Default)]
pub struct ResultSetOptions {
    pub urls: Vec<ResultSetRequest>,
}

#[derive(Debug, Default)]
pub(super) struct Accumulator {
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
    pub(super) unsupported: crate::cohort::UnsupportedCohortRows,
}

#[derive(Debug, Default)]
pub(super) struct Stats {
    pub(super) result_sets: usize,
    pub(super) result_sets_resumed: usize,
    pub(super) result_sets_empty: usize,
    pub(super) failures: Vec<String>,
    pub(super) rows: usize,
    pub(super) rows_with_grade: usize,
    pub(super) rows_without_grade: usize,
    pub(super) rows_without_school: usize,
    pub(super) rows_without_name: usize,
    pub(super) rows_school_named: usize,
    pub(super) rows_without_sport: usize,
    pub(super) events: usize,
    pub(super) skipped_lines: usize,
    pub(super) region_mismatch: usize,
    pub(super) school_resolved: HashMap<&'static str, usize>,
    pub(super) unresolved: HashMap<String, usize>,
}

pub async fn collect(
    ctx: &AdapterContext<'_>,
    options: &ResultSetOptions,
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER, "result rows");
    let (requests_before, cache_before) = stats_of(ctx).await;
    if options.urls.is_empty() {
        report.note("no result-set URLs supplied; nothing was requested".to_string());
        return Ok(report);
    }
    let mut run = Run {
        index: SchoolIndex::from_schools(&consolidated_schools(ctx)?),
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        done: ctx.store.journal_keys(PHASE)?,
        pending: Vec::new(),
    };
    for request in &options.urls {
        match ResultSetRef::parse(&request.url)
            .or_else(|| ResultSetRef::parse_with_jurisdiction(&request.url, request.jurisdiction))
        {
            Some(reference) => run.read(ctx, &reference).await,
            None => run.reject(&request.url),
        }
    }
    let counts = append(ctx, run.accumulated, run.pending)?;
    finish(
        ctx,
        &mut report,
        &run.stats,
        counts,
        requests_before,
        cache_before,
    )
    .await?;
    Ok(report)
}

async fn finish(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    stats: &Stats,
    counts: EntityCounts,
    requests_before: u64,
    cache_before: u64,
) -> CrawlResult<()> {
    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = u64::try_from(stats.rows).map_err(|_| CrawlError::Arithmetic {
        detail: "result row count does not fit in u64".to_string(),
    })?;
    report.errors = u64::try_from(stats.failures.len()).map_err(|_| CrawlError::Arithmetic {
        detail: "failure count does not fit in u64".to_string(),
    })?;
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    note_result_sets(report, stats);
    note_rows(report, stats);
    note_resolution(report, stats);
    note_entities(report, &counts);
    report.note(format!(
        "unsupported cohort observations retained: {}",
        counts.unsupported_cohorts
    ));
    report::note_failures(report, stats);
    Ok(())
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct EntityCounts {
    pub(super) meets: usize,
    pub(super) events: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) performances: usize,
    pub(super) unsupported_cohorts: usize,
}

fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let schools: Vec<CanonicalSchool> =
        census_store::read::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail:
                "no consolidated schools: run `collect` and `consolidate` before the milesplit \
                     result-set route"
                    .to_string(),
        });
    }
    Ok(schools)
}

fn append(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
    entries: Vec<(String, serde_json::Value)>,
) -> CrawlResult<EntityCounts> {
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    let mut page = ctx.write_batch();
    accumulated.unsupported.append_to(&mut page)?;
    page.append_many(Table::Meets, &meets)?;
    page.append_many(Table::Events, &events)?;
    page.append_many(Table::Teams, &teams)?;
    page.append_many(Table::Athletes, &athletes)?;
    page.append_many(Table::Performances, &performances)?;
    for (key, payload) in &entries {
        page.journal_done(PHASE, key, payload)?;
    }
    page.commit()?;
    Ok(EntityCounts {
        meets: meets.len(),
        events: events.len(),
        teams: teams.len(),
        athletes: athletes.len(),
        performances: performances.len(),
        unsupported_cohorts: accumulated.unsupported.len(),
    })
}
