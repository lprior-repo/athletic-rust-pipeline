//! One roster unit of a state walk: the skip a blocked state earns, and the fetch-and-record
//! step every other unit runs.
//!
//! Split out of `sweep.rs` so that file stays inside the one-page budget.

use crate::net::Fetcher;
use crate::sources::milesplit::{Site, TeamRef};
use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use census_store::Store;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::roster;
use super::{record_roster, Shared};

#[derive(Clone)]
pub(super) struct RosterRun<'a> {
    pub(super) site: Site,
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) observed_on: &'a str,
    pub(super) school_year: SchoolYear,
    pub(super) refresh: bool,
}

/// One roster unit: a state already blocked drops it unfetched — it stays owed, so the rest of the
/// state costs the host nothing — and otherwise the roster is fetched, stored and folded into the
/// shared progress row.
pub(super) async fn roster_unit(
    fetcher: &Fetcher,
    store: &Store,
    shared: &Arc<Mutex<Shared>>,
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
    )
    .await;
    record_roster(shared, store, run.jurisdiction, team, outcome).await;
}
