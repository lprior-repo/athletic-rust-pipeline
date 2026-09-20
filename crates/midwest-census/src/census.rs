//! Collection orchestration: state → teams → rosters → canonical entities, resumable at every step.
//!
//! Resume model: the entity logs are append-only and the journal records each completed unit of work
//! (`<state>:<team_id>`). A run that is interrupted — or an operator who stops one deliberately —
//! re-invokes with the same arguments and only the unfinished units are fetched again. HTTP bodies
//! are additionally cached on disk, so even a re-fetch costs no network traffic unless `--refresh`.

use crate::model::{CanonicalAthlete, CanonicalSchool, CanonicalTeam, SchoolYear};
use crate::net::{FetchOptions, Fetcher};
use crate::sources::milesplit::{self, Roster, Site, TeamRef};
use crate::store::{Store, Table};
use anyhow::{bail, Context, Result};
use futures::stream::{self, StreamExt};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone)]
pub struct CollectOptions {
    pub states: Vec<String>,
    pub limit_per_state: Option<usize>,
    pub concurrency: usize,
    /// How many state hosts to walk at once. Each host is still limited to one request at a time by
    /// the fetcher's per-host gate, so this only removes idle time between states.
    pub state_concurrency: usize,
    pub refresh: bool,
    pub school_year: SchoolYear,
    pub observed_on: String,
}

impl Default for CollectOptions {
    fn default() -> Self {
        Self {
            states: milesplit::SITES
                .iter()
                .map(|site| site.state.to_string())
                .collect(),
            limit_per_state: None,
            concurrency: 4,
            state_concurrency: 4,
            refresh: false,
            school_year: SchoolYear(2026),
            observed_on: crate::net::today_iso(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StateProgress {
    pub state: String,
    pub teams: usize,
    pub rosters_done: usize,
    pub rosters_skipped: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
    pub class_of_2027_boys: usize,
    pub class_of_2027_girls: usize,
    pub empty_rosters: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectReport {
    pub states: Vec<StateProgress>,
    pub teams_total: usize,
    pub rosters_fetched: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    pub requests: u64,
    pub cache_hits: u64,
    pub errors: u64,
    pub elapsed_seconds: f64,
}

/// Team index phase key.
fn teams_phase(state: &str) -> String {
    format!("milesplit_teams_{}", state.to_ascii_lowercase())
}

/// Roster phase key.
fn rosters_phase(state: &str) -> String {
    format!("milesplit_rosters_{}", state.to_ascii_lowercase())
}

/// Fetch (or read the cached copy of) one state's team index.
pub async fn collect_state_teams(
    fetcher: &Fetcher,
    store: &Store,
    state: &str,
    refresh: bool,
) -> Result<Vec<TeamRef>> {
    let site = milesplit::site_for_state(state)?;
    let phase = teams_phase(state);
    let known = store.journal_keys(&phase)?;
    if !refresh && known.contains(state) {
        // Team index already collected: reuse the journaled copy from the HTTP cache.
        let options = FetchOptions::default();
        let teams = milesplit::fetch_team_index(fetcher, site, &options).await?;
        return Ok(teams);
    }
    let options = FetchOptions {
        refresh,
        ..Default::default()
    };
    let teams = milesplit::fetch_team_index(fetcher, site, &options).await?;
    store.journal_done(
        &phase,
        state,
        &serde_json::json!({ "teams": teams.len(), "host": site.host }),
    )?;
    info!(state, teams = teams.len(), "team index collected");
    Ok(teams)
}

struct Shared {
    athletes: usize,
    co2027: usize,
    co2027_boys: usize,
    co2027_girls: usize,
    empty: usize,
    rosters: usize,
    errors: Vec<String>,
}

/// Walk every team roster for one state (resumable), emitting canonical entities.
pub async fn collect_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    state: &str,
) -> Result<StateProgress> {
    let site: Site = milesplit::site_for_state(state)?;
    let phase = rosters_phase(state);
    let done = store.journal_keys(&phase)?;
    let pending: Vec<TeamRef> = teams
        .iter()
        .filter(|team| !done.contains(&format!("{}:{}", state, team.id)))
        .cloned()
        .collect();
    let skipped = teams.len() - pending.len();

    let shared = Arc::new(Mutex::new(Shared {
        athletes: 0,
        co2027: 0,
        co2027_boys: 0,
        co2027_girls: 0,
        empty: 0,
        rosters: 0,
        errors: Vec::new(),
    }));

    let per_state_limit = options.limit_per_state;
    let working: Vec<TeamRef> = match per_state_limit {
        Some(limit) => pending.into_iter().take(limit).collect(),
        None => pending,
    };

    let fetcher_ref = fetcher;
    let store_ref = store;
    let observed_on = options.observed_on.clone();
    let school_year = options.school_year;
    let refresh = options.refresh;
    let concurrency = options.concurrency.max(1);

    stream::iter(working.into_iter().map(|team| {
        let shared = Arc::clone(&shared);
        let site = site;
        let observed_on = observed_on.clone();
        async move {
            let outcome = fetch_and_store_roster(
                fetcher_ref,
                store_ref,
                &site,
                &team,
                school_year,
                &observed_on,
                refresh,
            )
            .await;
            let mut guard = shared.lock().await;
            match outcome {
                Ok(roster) => {
                    guard.rosters += 1;
                    if roster.athletes.is_empty() {
                        guard.empty += 1;
                    }
                    let co2027 = roster
                        .athletes
                        .iter()
                        .filter(|athlete| athlete.grad_year == crate::model::GradYear::CO2027)
                        .count();
                    guard.athletes += roster.athletes.len();
                    guard.co2027 += co2027;
                    guard.co2027_boys += roster
                        .athletes
                        .iter()
                        .filter(|athlete| {
                            athlete.grad_year == crate::model::GradYear::CO2027
                                && athlete.gender == crate::model::Gender::Boys
                        })
                        .count();
                    guard.co2027_girls += roster
                        .athletes
                        .iter()
                        .filter(|athlete| {
                            athlete.grad_year == crate::model::GradYear::CO2027
                                && athlete.gender == crate::model::Gender::Girls
                        })
                        .count();
                    let _ = store_ref.journal_done(
                        &format!("milesplit_rosters_{}", state.to_ascii_lowercase()),
                        &format!("{}:{}", state, team.id),
                        &serde_json::json!({
                            "team": team.name,
                            "athletes": roster.athletes.len(),
                            "co2027": co2027,
                        }),
                    );
                }
                Err(error) => {
                    guard.errors.push(format!("{}: {error:#}", team.url));
                }
            }
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<()>>()
    .await;

    let guard = shared.lock().await;
    Ok(StateProgress {
        state: state.to_string(),
        teams: teams.len(),
        rosters_done: guard.rosters,
        rosters_skipped: skipped,
        athletes: guard.athletes,
        class_of_2027: guard.co2027,
        class_of_2027_boys: guard.co2027_boys,
        class_of_2027_girls: guard.co2027_girls,
        empty_rosters: guard.empty,
        errors: guard.errors.iter().take(5).cloned().collect(),
    })
}

async fn fetch_and_store_roster(
    fetcher: &Fetcher,
    store: &Store,
    site: &Site,
    team: &TeamRef,
    school_year: SchoolYear,
    observed_on: &str,
    refresh: bool,
) -> Result<Roster> {
    let options = FetchOptions {
        refresh,
        allow_not_found: true,
        headers: Vec::new(),
    };
    let roster = milesplit::fetch_roster(fetcher, team, &options).await?;
    let (school, athletes, teams) =
        milesplit::roster_entities(&roster, site.state, school_year, observed_on, site);
    store.append(Table::Schools, &school)?;
    if !teams.is_empty() {
        store.append_many(Table::Teams, &teams)?;
    }
    if !athletes.is_empty() {
        store.append_many(Table::Athletes, &athletes)?;
    }
    Ok(roster)
}

/// Full MileSplit walk across the requested states.
///
/// States are walked concurrently (each state is a different host) while every individual host keeps
/// its one-request-at-a-time, rate-limited discipline. Progress is journaled per state and per roster,
/// so an interrupted run resumes without refetching anything already collected.
pub async fn collect_milesplit(
    fetcher: &Fetcher,
    store: &Store,
    options: &CollectOptions,
) -> Result<CollectReport> {
    let started = std::time::Instant::now();
    let state_concurrency = options.state_concurrency.max(1);

    let results: Vec<(String, Result<StateProgress>)> =
        stream::iter(options.states.iter().cloned().map(|state| async move {
            let outcome = async {
                let teams = collect_state_teams(fetcher, store, &state, options.refresh)
                    .await
                    .with_context(|| format!("collecting {state} team index"))?;
                collect_state_rosters(fetcher, store, &teams, options, &state).await
            }
            .await;
            (state, outcome)
        }))
        .buffer_unordered(state_concurrency)
        .collect()
        .await;

    let mut report = CollectReport {
        states: Vec::new(),
        teams_total: 0,
        rosters_fetched: 0,
        athletes_total: 0,
        class_of_2027_total: 0,
        requests: 0,
        cache_hits: 0,
        errors: 0,
        elapsed_seconds: 0.0,
    };
    let mut failures = Vec::new();
    for (state, outcome) in results {
        match outcome {
            Ok(progress) => {
                report.teams_total += progress.teams;
                report.rosters_fetched += progress.rosters_done;
                report.athletes_total += progress.athletes;
                report.class_of_2027_total += progress.class_of_2027;
                report.errors += progress.errors.len() as u64;
                info!(
                    state,
                    teams = progress.teams,
                    rosters = progress.rosters_done,
                    co2027 = progress.class_of_2027,
                    "state complete"
                );
                report.states.push(progress);
            }
            Err(error) => failures.push(format!("{state}: {error:#}")),
        }
    }
    report
        .states
        .sort_by(|left, right| left.state.cmp(&right.state));
    let stats = fetcher.stats().await;
    report.requests = stats.requests;
    report.cache_hits = stats.cache_hits;
    report.elapsed_seconds = started.elapsed().as_secs_f64();
    if !failures.is_empty() {
        // Report what completed before failing: the caller keeps its journal and can re-run.
        bail!(
            "{} state(s) failed after {} rosters: {}",
            failures.len(),
            report.rosters_fetched,
            failures.join("; ")
        );
    }
    Ok(report)
}

/// Drop `professional_email` values that sit on consumer mail domains from a consolidated snapshot,
/// returning how many rows were withheld. Rows keep every other field.
fn scrub_consumer_emails(path: &Path) -> Result<usize> {
    let raw = std::fs::read_to_string(path)?;
    let mut withheld = 0usize;
    let mut rewritten = String::with_capacity(raw.len());
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut row: serde_json::Value =
            serde_json::from_str(line).context("re-reading a consolidated coach row")?;
        let is_personal = row
            .get("professional_email")
            .and_then(|value| value.as_str())
            .is_some_and(|email| crate::model::professional_email(email).is_none());
        if is_personal {
            row["professional_email"] = serde_json::Value::Null;
            withheld += 1;
        }
        rewritten.push_str(&serde_json::to_string(&row)?);
        rewritten.push('\n');
    }
    std::fs::write(path, rewritten)?;
    Ok(withheld)
}

/// Merge append logs into snapshots under `out/`, returning per-table counts.
pub fn consolidate(store: &Store) -> Result<Vec<(String, usize)>> {
    let out = store.out_dir();
    std::fs::create_dir_all(&out)?;
    let mut counts = Vec::new();
    counts.push((
        "schools".to_string(),
        store.consolidate::<CanonicalSchool>(Table::Schools, &out.join("schools.jsonl"))?,
    ));
    counts.push((
        "teams".to_string(),
        store.consolidate::<CanonicalTeam>(Table::Teams, &out.join("teams.jsonl"))?,
    ));
    let coaches_path = out.join("coaches.jsonl");
    counts.push((
        "coaches".to_string(),
        store.consolidate::<crate::model::CanonicalCoach>(Table::Coaches, &coaches_path)?,
    ));
    // Contract enforcement at the single choke point: the shipped coach projection never carries a
    // personal mailbox, whichever adapter accepted one.
    let withheld = scrub_consumer_emails(&coaches_path)?;
    counts.push(("coaches_email_withheld".to_string(), withheld));
    counts.push((
        "athletes".to_string(),
        store.consolidate::<CanonicalAthlete>(Table::Athletes, &out.join("athletes.jsonl"))?,
    ));
    counts.push((
        "meets".to_string(),
        store.consolidate::<crate::model::CanonicalMeet>(Table::Meets, &out.join("meets.jsonl"))?,
    ));
    counts.push((
        "events".to_string(),
        store.consolidate::<crate::model::CanonicalEvent>(
            Table::Events,
            &out.join("events.jsonl"),
        )?,
    ));
    counts.push((
        "performances".to_string(),
        store.consolidate::<crate::model::CanonicalPerformance>(
            Table::Performances,
            &out.join("performances.jsonl"),
        )?,
    ));
    Ok(counts)
}
