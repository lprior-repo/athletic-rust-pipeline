use census_crawl::milesplit::{Site, TeamRef};
use census_crawl::net::Fetcher;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::SchoolYear;
use census_store::Store;
use tokio::sync::Mutex;

use super::roster;
use super::{record_roster, Shared};
use super::{CollectOptions, OwnedRosterSource};

#[derive(Clone, Copy)]
pub(super) enum RosterSource<'a> {
    Historical,
    Owned(&'a OwnedRosterSource<'a>),
}

impl RosterSource<'_> {
    pub(super) fn pending(
        self,
        store: &Store,
        teams: &[TeamRef],
        options: &CollectOptions,
        jurisdiction: census_domain::UsJurisdiction,
    ) -> CrawlResult<Vec<TeamRef>> {
        if matches!(self, Self::Historical) {
            return Ok(super::pending_rosters(
                store,
                teams,
                jurisdiction,
                options.school_year,
                options.revision,
            )?);
        }
        owned_pending_rosters(teams)
    }
}

fn owned_pending_rosters(teams: &[TeamRef]) -> CrawlResult<Vec<TeamRef>> {
    crate::census::scope::require_roster_inputs(teams)?;
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(teams.len())
        .map_err(|_| super::team_resource(teams.len()))?;
    pending.extend_from_slice(teams);
    Ok(pending)
}

pub(super) struct RosterRun<'a> {
    pub(super) site: Site,
    pub(super) observed_on: &'a str,
    pub(super) school_year: SchoolYear,
    pub(super) refresh: bool,
    pub(super) revision: std::num::NonZeroU32,
}

#[tracing::instrument(skip_all, fields(team = team.id))]
pub(super) async fn roster_unit(
    fetcher: &Fetcher,
    store: &Store,
    shared: &Mutex<Shared>,
    team: &TeamRef,
    run: &RosterRun<'_>,
    source: RosterSource<'_>,
) {
    let outcome = match admission(shared).await {
        Ok(()) => match source {
            RosterSource::Historical => roster::fetch_and_store(fetcher, store, team, run).await,
            RosterSource::Owned(source) => source.fetch_and_store(fetcher, team, run).await,
        },
        Err(error) => Err(roster::retain_error(fetcher, store, team, run, error).await),
    };
    record_roster(shared, outcome).await;
}

async fn admission(shared: &Mutex<Shared>) -> CrawlResult<()> {
    let mut guard = shared.lock().await;
    if !guard.blocked {
        return Ok(());
    }
    guard.blocked_skipped =
        guard
            .blocked_skipped
            .checked_add(1)
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "blocked roster counter overflowed".to_string(),
            })?;
    Err(CrawlError::Invariant {
        detail: "roster remains owed: source admission stopped after a retained refusal"
            .to_string(),
    })
}

#[cfg(test)]
mod tests;
