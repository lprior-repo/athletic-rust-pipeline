//! One roster: fetch it, then append the canonical entities it carries.
//!
//! Split out of the parent module when the walk's block stop outgrew the repository's 300-line
//! budget; the request itself is unchanged, so the entity appends and the resume contract (a unit
//! counts as done only once its journal entry lands) stay exactly where they were.

use crate::net::{FetchOptions, Fetcher};
use crate::sources::milesplit::{self, Roster, Site, TeamRef};
use crate::sources::CrawlResult;
use crate::store::{Store, Table};
use census_domain::model::SchoolYear;

/// Fetch one roster and append its school, teams and athletes.
pub(super) async fn fetch_and_store(
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
