use census_crawl::milesplit::{Roster, TeamRef};
use census_domain::model::{Gender, GradYear, SchoolYear};
use census_store::{Store, StoreResult};

use super::rosters_phase;
use census_domain::UsJurisdiction;

pub(super) fn pending_rosters(
    store: &Store,
    teams: &[TeamRef],
    jurisdiction: UsJurisdiction,
    school_year: SchoolYear,
    revision: std::num::NonZeroU32,
) -> StoreResult<Vec<TeamRef>> {
    let state = jurisdiction.code();
    let done = store.journal_keys(&rosters_phase(jurisdiction, school_year, revision))?;
    Ok(teams
        .iter()
        .filter(|team| !done.contains(&format!("{}:{}", state, team.id)))
        .cloned()
        .collect())
}

pub(super) fn count_co2027(roster: &Roster) -> usize {
    roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count()
}

pub(super) fn count_cohort(roster: &Roster, gender: Gender) -> usize {
    roster
        .athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027 && athlete.gender == gender)
        .count()
}
