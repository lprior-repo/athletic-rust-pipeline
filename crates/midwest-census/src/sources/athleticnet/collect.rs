//! The run: the registry walk, the per-URL journal that lets a re-run resume, and the report
//! the collector prints.

use super::absorb::absorb;
use super::map::{Accumulator, Stats};
use super::parse::Bio;
use super::{
    read_registry, Options, Scope, Target, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, PARSE_VERSION, SCOPES,
};
use crate::net::FetchOptions;
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use crate::store::Table;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SchoolId, SourceRef,
};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

// -------------------------------------------------------------------------------------------------
// Collection
// -------------------------------------------------------------------------------------------------

/// Read every available result for the registry's athletes into the canonical store.
///
/// Strategy: one request per (athlete, sport) pair, journaled per URL so a re-run resumes; both
/// payloads are absorbed under one athlete so its teams and grades are minted once.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
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
        report,
    };
    absorb_targets(ctx, options, &targets, &source, &index, &done, &mut run).await?;

    let counts = store_accumulated(ctx, run.accumulated)?;
    let (requests_after, cache_after) = stats_of(ctx).await;
    run.report.rows = run.stats.athletes_absorbed;
    run.report.requests = requests_after.saturating_sub(requests_before);
    run.report.from_cache = cache_after.saturating_sub(cache_before);
    run.report.errors = run.stats.fetches_failed;
    note_stats(&mut run.report, &run.stats);
    run.report.note(format!(
        "canonical entities: schools {} meets {} teams {} athletes {} events {} performances {}",
        counts.schools,
        counts.meets,
        counts.teams,
        counts.athletes,
        counts.events,
        counts.performances
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
    report: AdapterReport,
}

/// Canonical entities written by one run, per table.
struct EntityCounts {
    schools: usize,
    meets: usize,
    teams: usize,
    athletes: usize,
    events: usize,
    performances: usize,
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

/// The URLs a previous run already journaled at the current parse version.
fn journaled_urls(ctx: &AdapterContext<'_>) -> CrawlResult<HashSet<String>> {
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

/// Absorb every pending (athlete, sport) payload into the run's accumulation.
async fn absorb_targets(
    ctx: &AdapterContext<'_>,
    options: &Options,
    targets: &[Target],
    source: &SourceRef,
    index: &SchoolIndex,
    done: &HashSet<String>,
    run: &mut RunState,
) -> CrawlResult<()> {
    for (processed, target) in targets.iter().enumerate() {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        run.stats.athletes_seen += 1;
        let mut absorbed_any = false;
        for scope in SCOPES {
            let url = format!(
                "{BIO_ENDPOINT}?athleteId={}&sport={}&level={HIGH_SCHOOL_LEVEL}",
                target.athlete_id,
                scope.parameter()
            );
            if done.contains(&url) {
                continue;
            }
            let Some(bio) = fetch_bio(ctx, target, scope, options, &url, run).await else {
                continue;
            };
            let rows = absorb(
                &bio,
                scope,
                target,
                source,
                &options.observed_on,
                index,
                &mut run.resolved,
                &mut run.stats,
                &mut run.accumulated,
            );
            absorbed_any |= rows > 0;
            ctx.store.journal_done(
                "athleticnet",
                &url,
                &json!({
                    "url": url,
                    "parser": PARSE_VERSION,
                    "parsed": true,
                    "athlete": target.athlete_id,
                    "sport": scope.parameter(),
                    "rows": rows,
                }),
            )?;
        }
        if absorbed_any {
            run.stats.athletes_absorbed += 1;
        }
    }
    Ok(())
}

/// Fetch and decode one (athlete, sport) payload, or `None` when it could not be read.
async fn fetch_bio(
    ctx: &AdapterContext<'_>,
    target: &Target,
    scope: Scope,
    options: &Options,
    url: &str,
    run: &mut RunState,
) -> Option<Bio> {
    let fetch_options = FetchOptions {
        refresh: options.refresh,
        allow_not_found: false,
        headers: vec![("Accept".to_string(), "application/json".to_string())],
    };
    let fetched = match ctx.fetcher.get(url, &fetch_options).await {
        Ok(fetched) => fetched,
        Err(error) => {
            run.stats.fetches_failed += 1;
            run.report
                .note(format!("athlete {}: {error}", target.athlete_id));
            return None;
        }
    };
    match serde_json::from_str(&fetched.text()) {
        Ok(bio) => Some(bio),
        Err(error) => {
            run.stats.fetches_failed += 1;
            run.report.note(format!(
                "athlete {} {}: body is not an athlete bio ({error})",
                target.athlete_id,
                scope.parameter()
            ));
            None
        }
    }
}

/// Append every entity the run accumulated and count what was written.
fn store_accumulated(
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
fn consolidated_index(ctx: &AdapterContext<'_>) -> CrawlResult<SchoolIndex> {
    let path = ctx.store.out_dir().join("schools.jsonl");
    if !path.exists() {
        return Ok(SchoolIndex::from_schools(&[]));
    }
    let schools: Vec<CanonicalSchool> = crate::report::read_rows(&path)?;
    Ok(SchoolIndex::from_schools(&schools))
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}
