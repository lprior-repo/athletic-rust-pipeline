//! Adapter sweep: one state's team index, then every roster in it, bounded and resumable.
//!
//! `collect_state_teams` fetches (or re-reads from the HTTP cache) one state's team index,
//! `collect_state_rosters` walks the rosters still owed for that state with at most
//! `options.concurrency` in flight, folding each outcome into one shared progress row, and
//! `collect_milesplit` walks the requested states concurrently — one host's pacing discipline per
//! state — failing only after reporting what completed first.

use crate::clock::{Clock, SystemClock};
use crate::net::{FetchOptions, Fetcher};
use crate::sources::milesplit::{self, Roster, Site, TeamRef};
use crate::sources::{CrawlError, CrawlResult};
use crate::store::{Store, Table};
use census_domain::model::{Gender, SchoolYear};
use census_domain::UsJurisdiction;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use super::aggregate::summarize_states;
use super::scope::{count_co2027, count_cohort, pending_rosters};
use super::{rosters_phase, teams_phase, CollectOptions, CollectReport, StateProgress};

/// Fetch (or read the cached copy of) one jurisdiction's team index.
#[tracing::instrument(skip(fetcher, store))]
pub async fn collect_state_teams(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    refresh: bool,
) -> CrawlResult<Vec<TeamRef>> {
    let site = Site::for_jurisdiction(jurisdiction);
    let state = jurisdiction.code();
    let phase = teams_phase(jurisdiction);
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
        &serde_json::json!({ "teams": teams.len(), "host": site.host() }),
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

/// Fold one roster outcome into the shared progress state and journal the completed unit of work.
///
/// A failed fetch is recorded instead of ending the walk, and a failed journal write is recorded
/// instead of dropped: either way the unit stays unfinished and the next run repeats it.
async fn record_roster(
    shared: &Mutex<Shared>,
    store: &Store,
    jurisdiction: UsJurisdiction,
    team: &TeamRef,
    outcome: CrawlResult<Roster>,
) {
    let roster = match outcome {
        Ok(roster) => roster,
        Err(error) => {
            let mut guard = shared.lock().await;
            guard.errors.push(format!("{}: {error}", team.url));
            return;
        }
    };
    let co2027 = count_co2027(&roster);
    let mut guard = shared.lock().await;
    guard.rosters = guard.rosters.saturating_add(1);
    if roster.athletes.is_empty() {
        guard.empty = guard.empty.saturating_add(1);
    }
    guard.athletes = guard.athletes.saturating_add(roster.athletes.len());
    guard.co2027 = guard.co2027.saturating_add(co2027);
    guard.co2027_boys = guard
        .co2027_boys
        .saturating_add(count_cohort(&roster, Gender::Boys));
    guard.co2027_girls = guard
        .co2027_girls
        .saturating_add(count_cohort(&roster, Gender::Girls));
    let payload = serde_json::json!({
        "team": team.name,
        "athletes": roster.athletes.len(),
        "co2027": co2027,
    });
    let journal = store.journal_done(
        &rosters_phase(jurisdiction),
        &format!("{}:{}", jurisdiction.code(), team.id),
        &payload,
    );
    if let Err(error) = journal {
        guard.errors.push(format!("{}: journal {error}", team.url));
    }
}

/// The progress row for one state, with the first few errors kept for the report.
fn progress_of(
    jurisdiction: UsJurisdiction,
    teams: usize,
    skipped: usize,
    shared: &Shared,
) -> StateProgress {
    StateProgress {
        jurisdiction,
        teams,
        rosters_done: shared.rosters,
        rosters_skipped: skipped,
        athletes: shared.athletes,
        class_of_2027: shared.co2027,
        class_of_2027_boys: shared.co2027_boys,
        class_of_2027_girls: shared.co2027_girls,
        empty_rosters: shared.empty,
        errors: shared.errors.iter().take(5).cloned().collect(),
    }
}

/// Walk every team roster for one jurisdiction (resumable), emitting canonical entities.
///
/// Bounded by the jurisdiction's team index, with at most `options.concurrency` rosters in flight.
#[tracing::instrument(skip(fetcher, store, teams, options))]
pub async fn collect_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<StateProgress> {
    let site: Site = Site::for_jurisdiction(jurisdiction);
    let (pending, skipped) = pending_rosters(store, teams, jurisdiction)?;
    let shared = Arc::new(Mutex::new(Shared {
        athletes: 0,
        co2027: 0,
        co2027_boys: 0,
        co2027_girls: 0,
        empty: 0,
        rosters: 0,
        errors: Vec::new(),
    }));

    let working: Vec<TeamRef> = match options.limit_per_state {
        Some(limit) => pending.into_iter().take(limit).collect(),
        None => pending,
    };

    let observed_on = options.observed_on.clone();
    let school_year = options.school_year;
    let refresh = options.refresh;
    let concurrency = options.concurrency.max(1);

    stream::iter(working.into_iter().map(|team| {
        let shared = Arc::clone(&shared);
        let observed_on = observed_on.clone();
        async move {
            let outcome = fetch_and_store_roster(
                fetcher,
                store,
                &site,
                &team,
                school_year,
                &observed_on,
                refresh,
            )
            .await;
            record_roster(&shared, store, jurisdiction, &team, outcome).await;
        }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<()>>()
    .await;

    let guard = shared.lock().await;
    Ok(progress_of(jurisdiction, teams.len(), skipped, &guard))
}

async fn fetch_and_store_roster(
    fetcher: &Fetcher,
    store: &Store,
    site: &Site,
    team: &TeamRef,
    school_year: SchoolYear,
    observed_on: &str,
    refresh: bool,
) -> CrawlResult<Roster> {
    let options = FetchOptions {
        refresh,
        allow_not_found: true,
        headers: Vec::new(),
    };
    let roster = milesplit::fetch_roster(fetcher, team, &options).await?;
    let (school, athletes, teams) =
        milesplit::roster_entities(&roster, school_year, observed_on, site);
    store.append(Table::Schools, &school)?;
    if !teams.is_empty() {
        store.append_many(Table::Teams, &teams)?;
    }
    if !athletes.is_empty() {
        store.append_many(Table::Athletes, &athletes)?;
    }
    Ok(roster)
}

/// Walk one jurisdiction: its team index, then every roster in it.
async fn walk_state(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    options: &CollectOptions,
) -> CrawlResult<StateProgress> {
    let teams = collect_state_teams(fetcher, store, jurisdiction, options.refresh)
        .await
        .map_err(|error| team_index_failure(jurisdiction, error))?;
    collect_state_rosters(fetcher, store, &teams, options, jurisdiction).await
}

/// Frame a team-index failure with the jurisdiction and the step, so a failure line names which half
/// of a jurisdiction's walk stopped.
fn team_index_failure(jurisdiction: UsJurisdiction, error: CrawlError) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("collecting {} team index: {error}", jurisdiction.code()),
    }
}

/// Full MileSplit walk across the requested states.
///
/// States are walked concurrently (each state is a different host) while every individual host keeps
/// its one-request-at-a-time, rate-limited discipline. Progress is journaled per state and per roster,
/// so an interrupted run resumes without refetching anything already collected.
#[tracing::instrument(skip(fetcher, store, options), fields(jurisdictions = options.jurisdictions.len()))]
pub async fn collect_milesplit(
    fetcher: &Fetcher,
    store: &Store,
    options: &CollectOptions,
) -> CrawlResult<CollectReport> {
    let started = SystemClock.now();
    let state_concurrency = options.state_concurrency.max(1);

    let results: Vec<(UsJurisdiction, CrawlResult<StateProgress>)> = stream::iter(
        options
            .jurisdictions
            .iter()
            .copied()
            .map(|jurisdiction| async move {
                let outcome = walk_state(fetcher, store, jurisdiction, options).await;
                (jurisdiction, outcome)
            }),
    )
    .buffer_unordered(state_concurrency)
    .collect()
    .await;

    let (mut report, failures) = summarize_states(results);
    let stats = fetcher.stats().await;
    report.requests = stats.requests;
    report.cache_hits = stats.cache_hits;
    report.elapsed_seconds = started.elapsed().as_secs_f64();
    if !failures.is_empty() {
        // Report what completed before failing: the caller keeps its journal and can re-run.
        return Err(CrawlError::Invariant {
            detail: format!(
                "{} state(s) failed after {} rosters: {}",
                failures.len(),
                report.rosters_fetched,
                failures.join("; ")
            ),
        });
    }
    Ok(report)
}
