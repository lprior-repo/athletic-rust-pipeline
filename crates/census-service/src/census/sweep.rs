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
mod roster;
mod units;

#[cfg(feature = "native-fault-injection")]
pub use self::roster::install_boundary_hook;

use crate::census::aggregate::summarize_states;

use self::units::{roster_unit, RosterRun};
use super::scope::pending_rosters;
use super::{
    rosters_phase, teams_phase, CollectOptions, CollectReport, StateProgress, TransportReport,
};

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
    let options = FetchOptions {
        refresh,
        ..Default::default()
    };
    let mut teams = milesplit::fetch_team_index(fetcher, site, &options).await?;
    let mut seen = HashSet::new();
    teams.retain(|team| seen.insert(team.id.clone()));
    if refresh || !known.contains(state) {
        store.journal_done(
            &phase,
            state,
            &serde_json::json!({ "teams": teams.len(), "host": site.host() }),
        )?;
    }
    info!(state, teams = teams.len(), "team index collected");
    Ok(teams)
}

struct Shared {
    errors: Vec<String>,
    blocked: bool,
    blocked_skipped: usize,
    refusals: access::RefusalRun,
}

fn progress_of(
    jurisdiction: UsJurisdiction,
    rosters_total: usize,
    summary: roster::Summary,
    shared: &mut Shared,
) -> CrawlResult<StateProgress> {
    let mut errors = std::mem::take(&mut shared.errors);
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

#[tracing::instrument(skip(fetcher, store, teams, options))]
pub async fn collect_state_rosters(
    fetcher: &Fetcher,
    store: &Store,
    teams: &[TeamRef],
    options: &CollectOptions,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<StateProgress> {
    let pending = pending_rosters(
        store,
        teams,
        jurisdiction,
        options.school_year,
        options.revision,
    )?;
    let shared = Mutex::new(Shared {
        errors: Vec::new(),
        blocked: false,
        blocked_skipped: 0,
        refusals: access::RefusalRun::default(),
    });
    let run = RosterRun {
        site: Site::for_jurisdiction(jurisdiction),
        observed_on: &options.observed_on,
        school_year: options.school_year,
        refresh: options.refresh,
        revision: options.revision,
    };
    stream::iter(&pending)
        .take(options.limit_per_state.map_or(pending.len(), |limit| limit))
        .for_each_concurrent(options.concurrency.max(1), |team| {
            roster_unit(fetcher, store, &shared, team, &run)
        })
        .await;
    let phase = rosters_phase(jurisdiction, options.school_year, options.revision);
    let summary = roster::summarize(store, &phase, teams)?;
    let mut guard = shared.lock().await;
    progress_of(jurisdiction, teams.len(), summary, &mut guard)
}

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

fn team_index_failure(jurisdiction: UsJurisdiction, error: CrawlError) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("collecting {} team index: {error}", jurisdiction.code()),
    }
}

#[tracing::instrument(skip(fetcher, store, options), fields(jurisdictions = options.jurisdictions.len()))]
pub async fn collect_milesplit(
    fetcher: &Fetcher,
    store: &Store,
    options: &CollectOptions,
) -> CrawlResult<CollectReport> {
    let started = Instant::now();
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
    report.elapsed_seconds = started.elapsed().as_secs_f64();
    let verified = u64::try_from(report.athletes_total).map_err(|_| CrawlError::Arithmetic {
        detail: "observed athlete count exceeds u64".to_string(),
    })?;
    report.transport = TransportReport::from_stats(&stats, verified);

    let observed = access::observed(fetcher, store).await;
    report.errors = report.errors.saturating_add(observed.failures);
    report.access_conditions = observed.conditions;
    report.blocked_hosts = observed.blocked_hosts;

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
