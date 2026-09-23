//! The run's dispatch, the resume reads that both routes share, and the accumulator append they
//! both write through.
//!
//! The bio walk itself lives in `walk`, split out when this file passed the repository's file
//! budget: what stays here is what the registry route dispatches to and what the meet route
//! imports.

use super::map::{Accumulator, Stats};
use super::{read_registry, Options, Target, BIO_ENDPOINT, PARSE_VERSION};
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SchoolId, SourceRef,
};
use census_store::Table;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod walk;

use walk::{absorb_targets, flush_batch};

// -------------------------------------------------------------------------------------------------
// Collection
// -------------------------------------------------------------------------------------------------

/// Read every available result for the registry's athletes into the canonical store.
///
/// Strategy: one request per (athlete, sport) pair, journaled per URL so a re-run resumes; both
/// payloads are absorbed under one athlete so its teams and grades are minted once.
///
/// A run whose options list meets (`--meets`) takes the whole-meet route instead: the `meet` module
/// pulls each listed meet whole, two requests per meet, and the registry is not read.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    if !options.meets.is_empty() {
        return super::meet::collect_meets(ctx, options).await;
    }
    let mut report = AdapterReport::new("athleticnet", "athletes");
    let (requests_before, cache_before) = stats_of(ctx).await;

    let targets = registry_targets(options, &mut report)?;
    let index = consolidated_index(ctx)?;
    let source = SourceRef::new("athleticnet", Some(BIO_ENDPOINT.to_string()));
    let done = journaled_urls(ctx)?;
    // One resolution per (state, school) for the whole run, so the counts are per school.
    let mut run = RunState {
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        pending: Vec::new(),
        batches: Vec::new(),
        report,
    };
    absorb_targets(ctx, options, &targets, &source, &index, &done, &mut run).await?;
    flush_batch(ctx, &mut run)?;

    let (requests_after, cache_after) = stats_of(ctx).await;
    run.report.rows = run.stats.athletes_absorbed;
    run.report.requests = requests_after.saturating_sub(requests_before);
    run.report.from_cache = cache_after.saturating_sub(cache_before);
    run.report.errors = run.stats.fetches_failed;
    note_stats(&mut run.report, &run.stats);
    run.report.note(format!(
        "canonical rows appended: schools {} meets {} teams {} athletes {} events {} performances \
         {}",
        appended_total(&run.batches, |batch| batch.schools),
        appended_total(&run.batches, |batch| batch.meets),
        appended_total(&run.batches, |batch| batch.teams),
        appended_total(&run.batches, |batch| batch.athletes),
        appended_total(&run.batches, |batch| batch.events),
        appended_total(&run.batches, |batch| batch.performances),
    ));
    run.report.note(
        "this source is outside the core scope (`report --core`): it is a reseller of results the \
         platform also gathers from governing bodies and timers, so the core comparison stays \
         independent of it",
    );
    Ok(run.report)
}

/// One run's sinks: what it accumulates, what it counts, and the report it narrates onto.
struct RunState {
    resolved: HashMap<String, SchoolId>,
    stats: Stats,
    accumulated: Accumulator,
    /// Units read since the last flush: the URL to journal and the payload to journal it with.
    pending: Vec<(String, Value)>,
    /// What each flush appended; the report's entity note sums them.
    batches: Vec<EntityCounts>,
    report: AdapterReport,
}

/// Canonical entities written by one run, per table.
pub(super) struct EntityCounts {
    pub(super) schools: usize,
    pub(super) meets: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) events: usize,
    pub(super) performances: usize,
}

/// Resolve the operator's registry into the run's targets, noting the ones that name no state.
fn registry_targets(options: &Options, report: &mut AdapterReport) -> CrawlResult<Vec<Target>> {
    let targets = read_registry(options)?;
    let without_state = targets.iter().filter(|t| t.state.is_none()).count();
    if without_state > 0 {
        report.note(format!(
            "{without_state} of {} targets carry no state; their rows are skipped, because a school \
             key without a state would merge same-named schools across states",
            targets.len()
        ));
    }
    Ok(targets)
}

/// Sum one table's counter over the run's flushes.
fn appended_total(batches: &[EntityCounts], counter: fn(&EntityCounts) -> usize) -> usize {
    batches.iter().map(counter).sum()
}

/// The URLs a previous run already journaled at the current parse version.
pub(super) fn journaled_urls(ctx: &AdapterContext<'_>) -> CrawlResult<HashSet<String>> {
    let payloads = ctx.store.journal_payloads("athleticnet")?;
    let version = u64::from(PARSE_VERSION);
    let done = payloads
        .into_iter()
        .filter(|entry| entry.get("parser").and_then(Value::as_u64) == Some(version))
        .filter(|entry| entry.get("parsed").and_then(Value::as_bool) == Some(true))
        .filter_map(|entry| entry.get("url").and_then(Value::as_str).map(str::to_string))
        .collect();
    Ok(done)
}

/// Append every entity the run accumulated and count what was written.
pub(super) fn store_accumulated(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
) -> CrawlResult<EntityCounts> {
    let schools: Vec<CanonicalSchool> = accumulated.schools.into_values().collect();
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    ctx.store.append_many(Table::Schools, &schools)?;
    ctx.store.append_many(Table::Meets, &meets)?;
    ctx.store.append_many(Table::Teams, &teams)?;
    ctx.store.append_many(Table::Athletes, &athletes)?;
    ctx.store.append_many(Table::Events, &events)?;
    ctx.store.append_many(Table::Performances, &performances)?;
    Ok(EntityCounts {
        schools: schools.len(),
        meets: meets.len(),
        teams: teams.len(),
        athletes: athletes.len(),
        events: events.len(),
        performances: performances.len(),
    })
}

/// Record what the run read, absorbed and refused, in the order the report reads.
fn note_stats(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "athletes: {} seen, {} absorbed, {} without a published grade, {} without a school entry, \
         {} whose gender the payload does not publish",
        stats.athletes_seen,
        stats.athletes_absorbed,
        stats.athletes_without_grade,
        stats.athletes_without_school,
        stats.gender_unknown
    ));
    report.note(format!(
        "result rows: {} seen, {} absorbed, {} without a mark token",
        stats.rows_seen, stats.rows_absorbed, stats.rows_no_mark
    ));
    report.note(format!(
        "skipped rows: {} with no season id, {} whose season publishes no indoor/outdoor split {}, \
         {} whose event id is absent from the payload's event dictionary, {} without a school \
         entry, {} without a meet entry, {} whose target carries no state",
        stats.rows_no_season,
        stats.rows_unknown_season.values().sum::<u64>(),
        if stats.rows_unknown_season.is_empty() {
            String::new()
        } else {
            format!("{:?}", stats.rows_unknown_season)
        },
        stats.rows_no_event,
        stats.rows_unknown_school,
        stats.rows_unknown_meet,
        stats.rows_without_state
    ));
    report.note(format!(
        "schools: {} resolved against the consolidated index, {} minted from this source",
        stats.schools_resolved, stats.schools_minted
    ));
}

/// The consolidated school index, when one exists. Athletic.net spans the whole country while the
/// index covers the platform's states, so a miss mints rather than skips.
pub(super) fn consolidated_index(ctx: &AdapterContext<'_>) -> CrawlResult<SchoolIndex> {
    let path = ctx.store.out_dir().join("schools.jsonl");
    if !path.exists() {
        return Ok(SchoolIndex::from_schools(&[]));
    }
    let schools: Vec<CanonicalSchool> = census_store::read::read_rows(&path)?;
    Ok(SchoolIndex::from_schools(&schools))
}

pub(super) async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}
