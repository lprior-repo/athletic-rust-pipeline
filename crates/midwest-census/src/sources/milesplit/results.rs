//! The `/raw` result-set route: one request per result set, the whole set, resumable by id.
//!
//! Measured request cost: **1 request per result set** — `samples/raw-oh-770621-rs1321880.txt` is the
//! whole result set of `RSID 1321880` in one 47,564 B body (80 rows, two sections, HTTP 200,
//! 2026-09-22T03:55:59Z), and the lane report records the same one-body shape for the other sampled
//! result sets. This route reads the `/raw` URLs it is given and walks nothing itself; the URLs come
//! from the meet's own results page, which lists every result file the meet has
//! (`MeetResultFile::raw_url`, `milesplit_results` provider arm) — so a whole meet is one page
//! request plus one request per result file, all of them ordinary HTML paths.
//!
//! Result sets are read sequentially, in the order supplied. The fetcher's per-host gate already
//! limits a host to one request at a time, so concurrency would only reorder the run — and the order
//! is what makes the journal and the report reproducible.
//!
//! Journal: one entry per result set, keyed `MeetID/RSID`, under a versioned phase — a parser change
//! that alters what an already-journaled result set yields bumps [`PHASE`], so those sets are read
//! again instead of being skipped as done.
//!
//! Layout: this file holds the entry point, the run's shared types and the table appends;
//! `results::run` holds the run state and the per-result-set read, `results::report` the notes the
//! run publishes.

use super::wire::ResultSetRef;
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam,
};
use census_store::Table;
use std::collections::HashMap;

mod report;
mod run;

use report::{note_entities, note_resolution, note_result_sets, note_rows};
use run::Run;

/// The journal phase of the result-set route, with the parser version encoded in the name.
const PHASE: &str = "milesplit_result_sets_v1";
/// The adapter name this route reports under.
const ADAPTER: &str = "milesplit_results";

/// What a result-set run is asked for: the `/raw` URLs to read, in the order to read them.
#[derive(Debug, Clone, Default)]
pub struct ResultSetOptions {
    pub urls: Vec<String>,
}

/// Canonical entities minted by one run, by table.
#[derive(Debug, Default)]
pub(super) struct Accumulator {
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
}

/// Run counters for the result-set route. Every field saturates: the counts are published in the run
/// notes, and a wrap would silently turn a large run into a small number there.
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

/// Read every supplied result set and write what they yield.
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
    };
    for entry in &options.urls {
        match ResultSetRef::parse(entry) {
            Some(reference) => run.read(ctx, &reference).await,
            None => run.reject(entry),
        }
    }
    let counts = append(ctx, run.accumulated)?;
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

/// Close the report: the fetcher's request and cache deltas, the row count, and what the run saw.
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
    report::note_failures(report, stats);
    Ok(())
}

/// The fetcher's request and cache counters, sampled before and after the walk.
async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

/// Canonical entity counts, for the run note.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct EntityCounts {
    pub(super) meets: usize,
    pub(super) events: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) performances: usize,
}

/// The consolidated schools this route resolves labels against. An empty file means `collect` and
/// `consolidate` have not been run yet, which is an operator error rather than a parse failure.
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

/// Append one batch per table and return what was written.
fn append(ctx: &AdapterContext<'_>, accumulated: Accumulator) -> CrawlResult<EntityCounts> {
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    ctx.store.append_many(Table::Meets, &meets)?;
    ctx.store.append_many(Table::Events, &events)?;
    ctx.store.append_many(Table::Teams, &teams)?;
    ctx.store.append_many(Table::Athletes, &athletes)?;
    ctx.store.append_many(Table::Performances, &performances)?;
    Ok(EntityCounts {
        meets: meets.len(),
        events: events.len(),
        teams: teams.len(),
        athletes: athletes.len(),
        performances: performances.len(),
    })
}
