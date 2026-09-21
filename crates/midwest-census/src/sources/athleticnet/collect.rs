//! The run: the registry walk, the per-URL journal that lets a re-run resume, and the report
//! the collector prints.

use super::absorb::absorb;
use super::map::{Accumulator, Stats};
use super::parse::Bio;
use super::{parse_targets, Options, BIO_ENDPOINT, HIGH_SCHOOL_LEVEL, PARSE_VERSION, SCOPES};
use crate::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SchoolId, SourceRef,
};
use crate::net::FetchOptions;
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::collections::{HashMap, HashSet};

// -------------------------------------------------------------------------------------------------
// Collection
// -------------------------------------------------------------------------------------------------

/// Read every available result for the registry's athletes into the canonical store.
///
/// Strategy: one request per (athlete, sport) pair, journaled per URL so a re-run resumes; both
/// payloads are absorbed under one athlete so its teams and grades are minted once.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("athleticnet", "athletes");
    let (requests_before, cache_before) = stats_of(ctx).await;

    let input = options.input.as_deref().context(
        "the athletic.net adapter needs --input with an athlete registry; its search endpoint is \
         disallowed by robots, so ids cannot be discovered by the tool",
    )?;
    let body = std::fs::read_to_string(input)
        .with_context(|| format!("reading the athlete registry {input}"))?;
    let targets = parse_targets(&body, &options.states)?;
    ensure!(
        !targets.is_empty(),
        "the athlete registry {input} lists no athlete ids"
    );
    let without_state = targets.iter().filter(|t| t.state.is_none()).count();
    if without_state > 0 {
        report.note(format!(
            "{without_state} of {} targets carry no state; their rows are skipped, because a school \
             key without a state would merge same-named schools across states",
            targets.len()
        ));
    }

    let index = consolidated_index(ctx)?;
    // One resolution per (state, school) for the whole run, so the counts are per school.
    let mut resolved: HashMap<String, SchoolId> = HashMap::new();
    let source = SourceRef::new("athleticnet", Some(BIO_ENDPOINT.to_string()));
    let done: HashSet<String> = ctx
        .store
        .journal_payloads("athleticnet")?
        .into_iter()
        .filter(|entry| {
            entry.get("parser").and_then(serde_json::Value::as_u64)
                == Some(u64::from(PARSE_VERSION))
                && entry.get("parsed").and_then(serde_json::Value::as_bool) == Some(true)
        })
        .filter_map(|entry| {
            entry
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();

    let mut stats = Stats::default();
    let mut accumulated = Accumulator::default();
    for (processed, target) in targets.iter().enumerate() {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        stats.athletes_seen += 1;
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
            let fetch_options = FetchOptions {
                refresh: options.refresh,
                allow_not_found: false,
                headers: vec![("Accept".to_string(), "application/json".to_string())],
            };
            let fetched = match ctx.fetcher.get(&url, &fetch_options).await {
                Ok(fetched) => fetched,
                Err(error) => {
                    stats.fetches_failed += 1;
                    report.note(format!("athlete {}: {error}", target.athlete_id));
                    continue;
                }
            };
            let bio: Bio = match serde_json::from_str(&fetched.text()) {
                Ok(bio) => bio,
                Err(error) => {
                    stats.fetches_failed += 1;
                    report.note(format!(
                        "athlete {} {}: body is not an athlete bio ({error})",
                        target.athlete_id,
                        scope.parameter()
                    ));
                    continue;
                }
            };
            let rows = absorb(
                &bio,
                scope,
                target,
                &source,
                &options.observed_on,
                &index,
                &mut resolved,
                &mut stats,
                &mut accumulated,
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
            stats.athletes_absorbed += 1;
        }
    }

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

    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = stats.athletes_absorbed;
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.errors = stats.fetches_failed;
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
    report.note(format!(
        "canonical entities: schools {} meets {} teams {} athletes {} events {} performances {}",
        schools.len(),
        meets.len(),
        teams.len(),
        athletes.len(),
        events.len(),
        performances.len()
    ));
    report.note(
        "this source is outside the core scope (`report --core`): it is a reseller of results the \
         platform also gathers from governing bodies and timers, so the core comparison stays \
         independent of it"
            .to_string(),
    );
    Ok(report)
}

/// The consolidated school index, when one exists. Athletic.net spans the whole country while the
/// index covers the platform's states, so a miss mints rather than skips.
fn consolidated_index(ctx: &AdapterContext<'_>) -> Result<SchoolIndex> {
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
