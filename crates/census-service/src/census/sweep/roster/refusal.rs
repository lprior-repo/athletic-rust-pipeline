use census_crawl::milesplit::{RosterRejection, TeamRef};
use census_crawl::net::FetchOutcome;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::serialized_digest;
use census_store::{Application, Store};

use super::super::rosters_phase;
use super::super::units::RosterRun;
use super::{roster_operation, Journal};

pub(crate) fn retain_refusal(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    error: &CrawlError,
) -> CrawlResult<Application> {
    let reason = error.to_string();
    let journal: Journal<&str, &FetchOutcome, Vec<RosterRejection>> = Journal {
        team_id: team.id.as_str(),
        school: None,
        year: run.school_year.get(),
        observed_on: run.observed_on,
        capture: None,
        refusal: Some(reason.as_str()),
        quarantine: None,
        rejected: Vec::new(),
        athletes: 0,
        teams: 0,
        co2027: 0,
        co2027_boys: 0,
        co2027_girls: 0,
    };
    let digest = serialized_digest(&(
        team.id.as_str(),
        run.school_year,
        run.observed_on,
        reason.as_str(),
    ))
    .map_err(|source| CrawlError::Canonical {
        table: "milesplit_roster".to_string(),
        source,
    })?;
    let state = run.site.jurisdiction().code();
    let operation = roster_operation(state, run.school_year, run.revision, team);
    let phase = rosters_phase(run.site.jurisdiction(), run.school_year, run.revision);
    let mut batch = store.write_batch();
    batch.journal_done(&phase, &format!("{state}:{}", team.id), &journal)?;
    Ok(batch.commit_once(&operation, &digest)?)
}
