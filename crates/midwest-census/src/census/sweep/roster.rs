//! One roster: fetch it, then append the canonical entities it carries.
//!
//! Split out of the parent module when the walk's block stop outgrew the repository's 300-line
//! budget; the request itself is unchanged, so the entity appends and the resume contract (a unit
//! counts as done only once its journal entry lands) stay exactly where they were.

use census_crawl::milesplit::{self, Roster, Site, TeamRef};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{observe_athletes_of, observe_schools_of, CrawlResult};
use census_domain::model::{SchoolYear, SourceNamespace};
use census_store::{Store, Table};

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
    observe_schools_of(
        store,
        &SourceNamespace::MilesplitSchool,
        std::slice::from_ref(&school),
        observed_on,
    )?;
    if !teams.is_empty() {
        store.append_many(Table::Teams, &teams)?;
    }
    if !athletes.is_empty() {
        store.append_many(Table::Athletes, &athletes)?;
        observe_athletes_of(
            store,
            &SourceNamespace::MilesplitAthlete,
            &athletes,
            std::slice::from_ref(&school),
            observed_on,
        )?;
    }
    Ok(roster)
}
