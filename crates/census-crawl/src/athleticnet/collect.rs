use super::map::{Accumulator, Stats};
use super::{read_registry, Options, Target, BIO_ENDPOINT, PARSE_VERSION, SCHOOL_KIND};
use crate::recording::RowBatch;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SchoolId, SourceNamespace,
};
use census_domain::school_index::SchoolIndex;
use census_store::Table;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod walk;

use walk::{absorb_targets, flush_batch};

pub(super) const PROFILE_PARSE_VERSION: u32 = 3;
pub(super) const PROFILE_ATTEMPT_PHASE: &str = "athleticnet_profile_attempts_v3";

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    if !options.meets.is_empty() {
        return super::meet::collect_meets(ctx, options).await;
    }
    let mut report = AdapterReport::new("athleticnet", "athletes");
    let (requests_before, cache_before) = stats_of(ctx).await;

    let targets = registry_targets(options, &mut report)?;
    let index = consolidated_index(ctx)?;
    let done = journaled_urls(ctx)?;
    let mut run = RunState {
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        pending: Vec::new(),
        batches: Vec::new(),
        report,
    };
    absorb_targets(ctx, options, &targets, &index, &done, &mut run).await?;
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
    run.report.note(format!(
        "unsupported graduation inference: {} raw grade/year observations retained for review",
        appended_total(&run.batches, |batch| batch.unsupported_cohorts),
    ));
    run.report.note(
        "this source is outside the core scope (`report --core`): it is a reseller of results the \
         platform also gathers from governing bodies and timers, so the core comparison stays \
         independent of it",
    );
    Ok(run.report)
}

struct RunState {
    resolved: HashMap<String, SchoolId>,
    stats: Stats,
    accumulated: Accumulator,
    pending: Vec<(String, Value)>,
    batches: Vec<EntityCounts>,
    report: AdapterReport,
}

pub(in crate::athleticnet) struct EntityCounts {
    pub(in crate::athleticnet) schools: usize,
    pub(in crate::athleticnet) meets: usize,
    pub(in crate::athleticnet) teams: usize,
    pub(in crate::athleticnet) athletes: usize,
    pub(in crate::athleticnet) events: usize,
    pub(in crate::athleticnet) performances: usize,
    pub(in crate::athleticnet) unsupported_cohorts: usize,
}

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

pub(in crate::athleticnet) fn appended_total(
    batches: &[EntityCounts],
    counter: fn(&EntityCounts) -> usize,
) -> usize {
    batches.iter().map(counter).sum()
}

pub(super) fn journaled_urls(ctx: &AdapterContext<'_>) -> CrawlResult<HashSet<String>> {
    let payloads = ctx.store.journal_payloads("athleticnet")?;
    let profile_attempts = ctx.store.journal_payloads(PROFILE_ATTEMPT_PHASE)?;
    let profiles: HashSet<_> = profile_attempts
        .into_iter()
        .filter(|entry| {
            entry.get("parser").and_then(Value::as_u64) == Some(u64::from(PROFILE_PARSE_VERSION))
                && entry.get("parsed").and_then(Value::as_bool) == Some(true)
        })
        .filter_map(|entry| entry.get("url").and_then(Value::as_str).map(str::to_string))
        .collect();
    let done = payloads
        .into_iter()
        .filter(|entry| {
            let version = match entry.get("url").and_then(Value::as_str) {
                Some(url) if url.starts_with(BIO_ENDPOINT) => PROFILE_PARSE_VERSION,
                _ => PARSE_VERSION,
            };
            entry.get("parser").and_then(Value::as_u64) == Some(u64::from(version))
        })
        .filter(|entry| entry.get("parsed").and_then(Value::as_bool) == Some(true))
        .filter(|entry| {
            entry
                .get("url")
                .and_then(Value::as_str)
                .is_some_and(|url| !url.starts_with(BIO_ENDPOINT) || profiles.contains(url))
        })
        .filter_map(|entry| entry.get("url").and_then(Value::as_str).map(str::to_string))
        .collect();
    Ok(done)
}

pub(super) fn store_accumulated(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
    page: &mut RowBatch<'_>,
) -> CrawlResult<EntityCounts> {
    let schools: Vec<CanonicalSchool> = accumulated.schools.into_values().collect();
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    page.append_many(Table::Schools, &schools)?;
    page.append_many(
        Table::SourceObservations,
        &ctx.school_observations(&SourceNamespace::athletic_net(SCHOOL_KIND), &schools),
    )?;
    page.append_many(Table::Meets, &meets)?;
    page.append_many(Table::Teams, &teams)?;
    page.append_many(Table::Athletes, &athletes)?;
    page.append_many(
        Table::SourceObservations,
        &ctx.athlete_observations(&athletes, &schools),
    )?;
    page.append_many(Table::Events, &events)?;
    page.append_many(Table::Performances, &performances)?;
    accumulated.unsupported.append_to(page)?;
    page.append_many(Table::SourceObservations, &accumulated.profile_observations)?;
    page.append_many(Table::ReviewCases, &accumulated.profile_reviews)?;
    Ok(EntityCounts {
        schools: schools.len(),
        meets: meets.len(),
        teams: teams.len(),
        athletes: athletes.len(),
        events: events.len(),
        performances: performances.len(),
        unsupported_cohorts: accumulated.unsupported.len(),
    })
}

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
