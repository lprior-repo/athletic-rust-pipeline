use census_crawl::milesplit::{self, Site, TeamRef};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::{Application, Store};
use futures::stream::{self, StreamExt};
use std::collections::HashSet;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::info;

mod access;
pub(crate) mod index;
pub(super) mod roster;
mod units;

#[cfg(feature = "native-fault-injection")]
pub use self::roster::install_boundary_hook;

use self::units::{roster_unit, RosterRun};
use super::scope::pending_rosters;
use super::{rosters_phase, CollectOptions, CollectReport, StateProgress, TransportReport};
use crate::census::aggregate::summarize_states;

const MAX_TEAMS: usize = 20_000;

#[tracing::instrument(skip(fetcher, store))]
pub async fn collect_state_teams(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    refresh: bool,
) -> CrawlResult<Vec<TeamRef>> {
    let site = Site::for_jurisdiction(jurisdiction);
    let read = match milesplit::fetch_team_index(
        fetcher,
        site,
        &FetchOptions {
            refresh,
            ..Default::default()
        },
    )
    .await
    {
        Ok(read) => read,
        Err(error) => return Err(index::retain_failure(store, jurisdiction, error)),
    };
    let teams = index::configure(store, jurisdiction, read)
        .map_err(|error| index::retain_failure(store, jurisdiction, error))?;
    info!(
        state = jurisdiction.code(),
        teams = teams.len(),
        "team index collected"
    );
    Ok(teams)
}

fn unique_teams(mut teams: Vec<TeamRef>) -> CrawlResult<Vec<TeamRef>> {
    team_bound(teams.len())?;
    let mut seen = HashSet::new();
    seen.try_reserve(teams.len())
        .map_err(|_| team_resource(teams.len()))?;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(teams.len())
        .map_err(|_| team_resource(teams.len()))?;
    selected.extend(teams.iter().map(|team| seen.insert(team.id.as_str())));
    drop(seen);
    let mut selected = selected.into_iter();
    teams.retain(|_| selected.next().is_some_and(core::convert::identity));
    Ok(teams)
}

fn team_bound(count: usize) -> CrawlResult<()> {
    if count > MAX_TEAMS {
        return Err(team_resource(count));
    }
    Ok(())
}

fn team_resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "indexed roster teams",
        requested,
        limit: MAX_TEAMS,
    }
}

struct Shared {
    errors: Vec<String>,
    blocked: bool,
    blocked_skipped: usize,
    refusals: access::RefusalRun,
}

fn shared(count: usize) -> CrawlResult<Mutex<Shared>> {
    let mut errors = Vec::new();
    errors
        .try_reserve_exact(count)
        .map_err(|_| team_resource(count))?;
    Ok(Mutex::new(Shared {
        errors,
        blocked: false,
        blocked_skipped: 0,
        refusals: access::RefusalRun::default(),
    }))
}

fn progress_of(
    jurisdiction: UsJurisdiction,
    rosters_total: usize,
    summary: roster::Summary,
    shared: &mut Shared,
) -> CrawlResult<StateProgress> {
    let mut errors = std::mem::take(&mut shared.errors);
    errors
        .try_reserve_exact(summary.errors.len())
        .map_err(|_| team_resource(rosters_total))?;
    errors.extend(summary.errors);
    Ok(StateProgress {
        jurisdiction,
        rosters_total,
        rosters_committed: summary.committed,
        rosters_remaining: roster::remaining(rosters_total, summary.committed, summary.held)?,
        rosters_skipped: summary.held,
        athletes: summary.athletes,
        class_of_2027: summary.co2027,
        class_of_2027_boys: summary.co2027_boys,
        class_of_2027_girls: summary.co2027_girls,
        errors,
        blocked: shared.blocked,
        blocked_skipped: shared.blocked_skipped,
        teams: rosters_total,
    })
}

#[tracing::instrument(skip_all, fields(state = jurisdiction.code()))]
pub async fn collect_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<StateProgress> {
    team_bound(teams.len())?;
    let pending = pending_rosters(
        store,
        teams,
        jurisdiction,
        options.school_year,
        options.revision,
    )?;
    let shared = shared(pending.len())?;
    let run = RosterRun {
        site: Site::for_jurisdiction(jurisdiction),
        observed_on: &options.observed_on,
        school_year: options.school_year,
        refresh: options.refresh,
        revision: options.revision,
    };
    stream::iter(&pending)
        .take(options.limit_per_state.map_or(pending.len(), |limit| limit))
        .for_each_concurrent(options.concurrency.clamp(1, 64), |team| {
            roster_unit(fetcher, store, &shared, team, &run)
        })
        .await;
    let phase = rosters_phase(jurisdiction, options.school_year, options.revision);
    let summary = roster::summarize(store, &phase, teams, jurisdiction, options.school_year)?;
    let progress = progress_of(
        jurisdiction,
        summary.total,
        summary,
        &mut *shared.lock().await,
    )?;
    index::guard_progress(store, jurisdiction, teams, progress)
}

#[tracing::instrument(skip_all, fields(state = jurisdiction.code()))]
async fn walk_state(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    options: &CollectOptions,
) -> CrawlResult<StateProgress> {
    let teams = collect_state_teams(fetcher, store, jurisdiction, options.refresh)
        .await
        .map_err(|error| CrawlError::Invariant {
            detail: format!("collecting {} team index: {error}", jurisdiction.code()),
        })?;
    collect_state_rosters(fetcher, store, &teams, options, jurisdiction).await
}

#[tracing::instrument(skip_all, fields(jurisdictions = options.jurisdictions.len()))]
pub async fn collect_milesplit(
    fetcher: &Fetcher,
    store: &Store,
    options: &CollectOptions,
) -> CrawlResult<CollectReport> {
    if options.jurisdictions.len() > UsJurisdiction::ALL.len() {
        return Err(team_resource(options.jurisdictions.len()));
    }
    let started = Instant::now();
    let results = collect_states(fetcher, store, options).await?;
    let (mut report, failures) = summarize_states(results);
    finish_report(fetcher, store, &mut report, started).await?;
    if !failures.is_empty() {
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

async fn collect_states(
    fetcher: &Fetcher,
    store: &Store,
    options: &CollectOptions,
) -> CrawlResult<Vec<(UsJurisdiction, CrawlResult<StateProgress>)>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(options.jurisdictions.len())
        .map_err(|_| team_resource(options.jurisdictions.len()))?;
    Ok(stream::iter(options.jurisdictions.iter().copied())
        .map(|jurisdiction| async move {
            (
                jurisdiction,
                walk_state(fetcher, store, jurisdiction, options).await,
            )
        })
        .buffer_unordered(options.state_concurrency.clamp(1, 51))
        .fold(rows, |mut rows, row| async move {
            rows.push(row);
            rows
        })
        .await)
}

async fn finish_report(
    fetcher: &Fetcher,
    store: &Store,
    report: &mut CollectReport,
    started: Instant,
) -> CrawlResult<()> {
    let stats = fetcher.stats().await;
    report.elapsed_seconds = started.elapsed().as_secs_f64();
    let verified = u64::try_from(report.athletes_total).map_err(|_| CrawlError::Arithmetic {
        detail: "observed athlete count exceeds u64".to_string(),
    })?;
    report.transport = TransportReport::from_stats(&stats, verified);
    let observed = access::observed(fetcher, store).await;
    report.errors = report
        .errors
        .checked_add(observed.failures)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "source access failure count overflowed".to_string(),
        })?;
    report.access_conditions = observed.conditions;
    report.blocked_hosts = observed.blocked_hosts;
    Ok(())
}

async fn record_roster(shared: &Mutex<Shared>, outcome: CrawlResult<Application>) {
    let mut guard = shared.lock().await;
    let refusal = match outcome {
        Ok(_) => access::Refusal::None,
        Err(error) => {
            let refusal = access::refusal(&error);
            guard.errors.push(error.to_string());
            refusal
        }
    };
    if guard.refusals.observe(refusal) {
        guard.blocked = true;
    }
}
