//! Adapter sweep: one state's team index, then every roster in it, bounded and resumable.
//!
//! `collect_state_teams` fetches (or re-reads from the HTTP cache) one state's team index,
//! `collect_state_rosters` walks the rosters still owed for that state with at most
//! `options.concurrency` in flight, folding each outcome into one shared progress row, and
//! `collect_milesplit` walks the requested states concurrently — one host's pacing discipline per
//! state — failing only after reporting what completed first, and stopping a state's walk on the
//! first hard access block (§69) instead of spending its remaining rosters on a host that has
//! already refused this client.
//!
//! One roster unit's work — the skip a blocked state earns, and the fetch-and-store step every other
//! unit runs — lives in `units`, so this file stays inside the one-page budget.

use census_crawl::milesplit::{self, Roster, Site, TeamRef};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::Gender;
use census_domain::UsJurisdiction;
use census_store::clock::{Clock, SystemClock};
use census_store::Store;
use futures::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

mod access;
mod roster;
mod units;

use self::units::{roster_unit, RosterRun};
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
    /// §69: set by the first roster outcome the host refused (HTTP 429), or by a contiguous run of
    /// page refusals that reads as a wall rather than as one unpublished roster.
    blocked: bool,
    /// Units the walk dropped unfetched because the block was already known.
    blocked_skipped: usize,
    /// The consecutive page refusals seen so far, which is what tells a wall from a dead page.
    refusals: access::RefusalRun,
}

/// Fold one roster outcome into the shared progress state and journal the completed unit of work.
///
/// A failed fetch is recorded instead of ending the walk — unless it is a hard access block, which
/// ends this state's walk (§69) — and a failed journal write is recorded instead of dropped: either
/// way the unit stays unfinished and the next run repeats it.
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
            if guard.refusals.observe(access::refusal(&error)) {
                guard.blocked = true;
            }
            guard.errors.push(format!("{}: {error}", team.url));
            return;
        }
    };
    let co2027 = count_co2027(&roster);
    let mut guard = shared.lock().await;
    // A roster the host served breaks any run of page refusals.
    guard.refusals.observe(access::Refusal::None);
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
        blocked: shared.blocked,
        blocked_skipped: shared.blocked_skipped,
    }
}

/// The walk's loop-invariant inputs: the site it reads, the state it is walking, and the observation
/// every roster row carries.
/// Walk every team roster for one jurisdiction (resumable), emitting canonical entities.
///
/// Bounded by the jurisdiction's team index, with at most `options.concurrency` rosters in flight.
/// A hard access block (§69) ends the state's requests: what is already in flight drains, every
/// later roster is counted as skipped instead of issued, and both stay owed for a later run.
#[tracing::instrument(skip(fetcher, store, teams, options))]
pub async fn collect_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<StateProgress> {
    let (pending, skipped) = pending_rosters(store, teams, jurisdiction)?;
    let shared = Arc::new(Mutex::new(Shared {
        athletes: 0,
        co2027: 0,
        co2027_boys: 0,
        co2027_girls: 0,
        empty: 0,
        rosters: 0,
        errors: Vec::new(),
        blocked: false,
        refusals: access::RefusalRun::default(),
        blocked_skipped: 0,
    }));
    let working: Vec<TeamRef> = match options.limit_per_state {
        Some(limit) => pending.into_iter().take(limit).collect(),
        None => pending,
    };
    let run = RosterRun {
        site: Site::for_jurisdiction(jurisdiction),
        jurisdiction,
        observed_on: &options.observed_on,
        school_year: options.school_year,
        refresh: options.refresh,
    };
    stream::iter(working.into_iter().map(|team| {
        let shared = Arc::clone(&shared);
        let run = run.clone();
        async move { roster_unit(fetcher, store, &shared, &team, &run).await }
    }))
    .buffer_unordered(options.concurrency.max(1))
    .collect::<Vec<()>>()
    .await;
    let guard = shared.lock().await;
    Ok(progress_of(jurisdiction, teams.len(), skipped, &guard))
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

    // §69: what the sources said about this client is recorded before the report goes back, so a
    // block the walk paid for is a row the next run reads instead of re-discovering.
    let observed = access::observed(fetcher, store).await;
    report.errors = report.errors.saturating_add(observed.failures);
    report.access_conditions = observed.conditions;
    report.blocked_hosts = observed.blocked_hosts;

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
