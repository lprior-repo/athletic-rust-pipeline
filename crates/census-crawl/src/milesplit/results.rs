use super::map::ProviderSchools;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult, UnresolvedCounters};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SourceObservation,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use std::collections::{HashMap, HashSet};

mod accumulator;
mod effects;
mod group;
mod pages;
mod report;
mod run;

use accumulator::pending_rows;
use effects::append_new;
pub use pages::{
    is_results_page, read_meet_pages, ListedResultFile, MeetPage, MeetPages, MISMATCH_LIMIT,
};
use report::{note_entities, note_failures};
use run::Run;

pub const RESULT_SET_PHASE: &str = "milesplit_result_sets_v5";
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
    pub(super) observations: HashMap<String, SourceObservation>,
    pub(super) retained: HashMap<String, serde_json::Value>,
    pub(super) pending: HashMap<&'static str, HashSet<String>>,
}

#[derive(Debug, Default)]
pub(super) struct Stats {
    pub(super) result_sets: usize,
    pub(super) result_sets_resumed: usize,
    pub(super) result_sets_empty: usize,
    pub(super) failures: Vec<String>,
    pub(super) rows: usize,
    pub(super) rows_with_cohort: usize,
    pub(super) rows_without_cohort: usize,
    pub(super) rows_without_school: usize,
    pub(super) rows_without_name: usize,
    pub(super) rows_without_sport: usize,
    pub(super) events: usize,
    pub(super) skipped_lines: usize,
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
    let schools = live_schools(ctx)?;
    let schools = ProviderSchools::from_schools(&schools);
    let window_rows = 5000usize;
    let mut run = Run {
        schools,
        owned: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        seen: HashSet::new(),
        pending: Vec::new(),
        meet_id: String::new(),
    };
    let groups = group::group_meets(&mut run, &options.urls);
    let mut total_counts = EntityCounts::default();
    for group in groups {
        group::run_group(ctx, &mut run, group, window_rows, &mut total_counts).await?;
    }
    finish(
        ctx,
        &mut report,
        &run.stats,
        total_counts,
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
    report.unresolved = Some(UnresolvedCounters {
        rows: u64::try_from(stats.rows_without_school).map_err(|_| CrawlError::Arithmetic {
            detail: "unresolved school row count does not fit in u64".to_string(),
        })?,
        labels: u64::try_from(stats.unresolved.len()).map_err(|_| CrawlError::Arithmetic {
            detail: "unresolved school label count does not fit in u64".to_string(),
        })?,
    });
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.note(format!(
        "owned result sets: {} projected, {} already journaled, {} empty; {} retained failures",
        stats.result_sets,
        stats.result_sets_resumed,
        stats.result_sets_empty,
        stats.failures.len(),
    ));
    report.note(format!(
        "owned individual observations: {}; published cohorts: {}; missing/invalid cohorts retained: {}; unresolved provider teamIDs: {}; absent raw sport metadata: {}",
        stats.rows, stats.rows_with_cohort, stats.rows_without_cohort,
        stats.rows_without_school, stats.rows_without_sport,
    ));
    report.note(format!(
        "exact unique MilesplitSchool provider bindings: {:?}; unresolved mappings: {:?}",
        stats.school_resolved, stats.unresolved,
    ));
    report.note("Source completeness remains capture-scoped and may be Unknown; projected observations do not assert canonical identity acceptance, a lifetime PR, source exhaustion or a census seal.");
    note_entities(report, &counts);
    report.note(format!(
        "unsupported cohort observations retained: {}",
        counts.unsupported_cohorts
    ));
    note_failures(report, stats);
    Ok(())
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.physical_requests(), stats.cache_hits)
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

fn live_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    ctx.store.scan(Table::Schools).map_err(CrawlError::Store)
}

fn append(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
    entries: Vec<(String, serde_json::Value)>,
) -> CrawlResult<EntityCounts> {
    if accumulated.pending.is_empty() && entries.is_empty() {
        return Ok(EntityCounts::default());
    }
    let pending = accumulated.pending;
    let meets = pending_rows(accumulated.meets, Table::Meets, &pending);
    let events = pending_rows(accumulated.events, Table::Events, &pending);
    let teams = pending_rows(accumulated.teams, Table::Teams, &pending);
    let athletes = pending_rows(accumulated.athletes, Table::Athletes, &pending);
    let performances = pending_rows(accumulated.performances, Table::Performances, &pending);
    let observations = pending_rows(
        accumulated.observations,
        Table::SourceObservations,
        &pending,
    );
    let unsupported_cohorts = accumulated
        .retained
        .values()
        .filter(|row| row["cohort"] != "published")
        .count();
    let mut page = ctx.write_batch();
    append_new(
        ctx.store,
        &mut page,
        Table::SourceObservations,
        observations,
    )?;
    accumulated
        .retained
        .iter()
        .filter(|(key, _)| {
            pending
                .get("retained")
                .is_some_and(|keys| keys.contains(*key))
        })
        .try_for_each(|(key, payload)| journal_changed(ctx, &mut page, key, payload))?;
    let meets = append_new(ctx.store, &mut page, Table::Meets, meets)?;
    let events = append_new(ctx.store, &mut page, Table::Events, events)?;
    let teams = append_new(ctx.store, &mut page, Table::Teams, teams)?;
    let athletes = append_new(ctx.store, &mut page, Table::Athletes, athletes)?;
    let performances = append_new(ctx.store, &mut page, Table::Performances, performances)?;
    entries
        .iter()
        .try_for_each(|(key, payload)| journal_changed(ctx, &mut page, key, payload))?;
    page.commit()?;
    Ok(EntityCounts {
        meets,
        events,
        teams,
        athletes,
        performances,
        unsupported_cohorts,
    })
}

fn journal_changed(
    ctx: &AdapterContext<'_>,
    page: &mut crate::recording::RowBatch<'_>,
    key: &str,
    payload: &serde_json::Value,
) -> CrawlResult<()> {
    match ctx.store.journal_payload(RESULT_SET_PHASE, key)? {
        Some(prior) if &prior == payload => {}
        Some(_) => {
            return Err(CrawlError::Invariant {
                detail: format!("content-bound result projection receipt changed: {key}"),
            })
        }
        None => page.journal_done(RESULT_SET_PHASE, key, payload)?,
    }
    Ok(())
}
