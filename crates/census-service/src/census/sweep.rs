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

pub(crate) mod access;
mod collect;
pub(crate) mod index;
pub(crate) use index::retain_failure as retain_team_index_failure;
mod owned;
pub mod profiles;
pub use collect::{collect_owned_state_rosters, collect_state_rosters};
pub(super) mod roster;
mod units;
pub use owned::OwnedRosterSource;

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
) -> Result<Vec<TeamRef>, access::SourceAccessFailure> {
    let options = FetchOptions {
        refresh,
        ..Default::default()
    };
    let read =
        milesplit::fetch_team_index(fetcher, Site::for_jurisdiction(jurisdiction), &options).await;
    accept_state_team_index(fetcher, store, jurisdiction, read).await
}

pub async fn accept_state_team_index(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    read: CrawlResult<milesplit::TeamIndexRead>,
) -> Result<Vec<TeamRef>, access::SourceAccessFailure> {
    let teams = match read.and_then(|read| index::configure(store, jurisdiction, read)) {
        Ok(teams) => teams,
        Err(error) => return Err(index::retain_failure(fetcher, store, jurisdiction, error).await),
    };
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
) -> Result<CollectReport, access::SourceAccessFailure> {
    if options.jurisdictions.len() > UsJurisdiction::ALL.len() {
        return Err(team_resource(options.jurisdictions.len()).into());
    }
    let started = Instant::now();
    let results = collect_states(fetcher, store, options).await?;
    let (mut report, failures) = summarize_states(results);
    let access = finish_report(fetcher, store, &mut report, started).await;
    let outcome = if failures.is_empty() {
        Ok(report)
    } else {
        Err(CrawlError::Invariant {
            detail: format!(
                "{} state(s) failed after {} rosters: {}",
                failures.len(),
                report.rosters_fetched,
                failures.join("; ")
            ),
        })
    };
    match (outcome, access) {
        (Ok(report), Ok(())) => Ok(report),
        (Err(primary), Ok(())) | (Ok(_), Err(primary)) => {
            Err(access::SourceAccessFailure::Primary(primary))
        }
        (Err(primary), Err(secondary)) => {
            Err(access::SourceAccessFailure::retention(primary, secondary))
        }
    }
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
    let observed = access::observed(fetcher, store).await?;
    report.errors = report
        .errors
        .checked_add(observed.failures)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "source access failure count overflowed".to_string(),
        })?;
    report.access_conditions = observed.conditions;
    report.blocked_hosts = observed.blocked_hosts;
    report.evidence_losses = observed.evidence_losses;
    Ok(())
}

async fn record_roster(shared: &Mutex<Shared>, outcome: Result<Application, roster::Failure>) {
    let mut guard = shared.lock().await;
    let refusal = match outcome {
        Ok(_) => access::Refusal::None,
        Err(error) => {
            let refusal = access::refusal(error.primary());
            guard.errors.push(error.to_string());
            refusal
        }
    };
    if guard.refusals.observe(refusal) {
        guard.blocked = true;
    }
}
