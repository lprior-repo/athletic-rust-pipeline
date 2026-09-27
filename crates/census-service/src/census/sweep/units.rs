use census_crawl::milesplit::{Site, TeamRef};
use census_crawl::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use tokio::sync::Mutex;

use super::roster;
use super::{record_roster, Shared};

pub(super) struct RosterRun<'a> {
    pub(super) site: Site,
    pub(super) observed_on: &'a str,
    pub(super) school_year: SchoolYear,
    pub(super) refresh: bool,
    pub(super) revision: std::num::NonZeroU32,
}

pub(super) async fn roster_unit(
    fetcher: &Fetcher,
    store: &Store,
    shared: &Mutex<Shared>,
    team: &TeamRef,
    run: &RosterRun<'_>,
) {
    {
        let mut guard = shared.lock().await;
        if guard.blocked {
            guard.blocked_skipped = guard.blocked_skipped.saturating_add(1);
            return;
        }
    }
    let outcome = roster::fetch_and_store(
        fetcher,
        store,
        &run.site,
        team,
        run.school_year,
        run.observed_on,
        run.refresh,
        run.revision,
    )
    .await;
    record_roster(shared, outcome).await;
}
