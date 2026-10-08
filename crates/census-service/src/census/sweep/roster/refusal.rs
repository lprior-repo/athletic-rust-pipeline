use census_crawl::milesplit::{RosterOutcome, TeamRef};
use census_crawl::{CollectionDisposition, CrawlError, CrawlResult};
use census_domain::model::serialized_digest;
use census_store::{Application, Store};
use serde::Serialize;

use super::super::rosters_phase;
use super::super::units::RosterRun;
use super::counts::Counts;
use super::journal::{roster_journal, roster_operation, Journal};

pub(in crate::census::sweep) fn retain_error(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    error: CrawlError,
) -> CrawlError {
    match retain_refusal(store, team, run, &error) {
        Ok(_) => error,
        Err(persistence) => combined(error, persistence),
    }
}

pub(super) fn retain_capture_error(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    read: &RosterOutcome,
    error: CrawlError,
) -> CrawlError {
    match captured(store, team, run, read, &error) {
        Ok(_) => error,
        Err(persistence) => combined(error, persistence),
    }
}

pub(crate) fn retain_refusal(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    error: &CrawlError,
) -> CrawlResult<Application> {
    let mut journal = match super::prior(store, team, run)? {
        Some(journal) => journal,
        None => empty(team, run),
    };
    journal.refusal = Some(error.to_string());
    journal.disposition = disposition(error);
    persist(store, team, run, &journal)
}

fn captured(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    read: &RosterOutcome,
    error: &CrawlError,
) -> CrawlResult<Application> {
    let prior = super::prior(store, team, run)?;
    let counts = Counts::default();
    let mut journal = roster_journal(team, read, None, run, &counts);
    if let Some(prior) = &prior {
        super::preserve_counts(&mut journal, prior);
        journal.school = prior.school.as_deref();
    }
    let reason = error.to_string();
    journal.refusal = Some(reason.as_str());
    journal.disposition = disposition(error);
    persist(store, team, run, &journal)
}

fn persist(
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
    journal: &impl Serialize,
) -> CrawlResult<Application> {
    let digest = serialized_digest(journal).map_err(|source| CrawlError::Canonical {
        table: "milesplit_roster".to_string(),
        source,
    })?;
    let state = run.site.jurisdiction().code();
    let operation = format!(
        "{}:refusal:{digest}",
        roster_operation(state, run.school_year, run.revision, team)
    );
    let phase = rosters_phase(run.site.jurisdiction(), run.school_year, run.revision);
    let key = format!("{state}:{}", team.id);
    let mut batch = store.write_batch();
    batch.journal_done(&phase, &key, journal)?;
    let application = batch.commit_once(&operation, &digest)?;
    if !application.written() {
        store.journal_done(&phase, &key, journal)?;
    }
    Ok(application)
}

fn disposition(error: &CrawlError) -> CollectionDisposition {
    match error {
        CrawlError::Resource { .. } | CrawlError::Arithmetic { .. } => {
            CollectionDisposition::Partial
        }
        _ => match super::super::access::refusal(error) {
            super::super::access::Refusal::Host | super::super::access::Refusal::Page => {
                CollectionDisposition::Blocked
            }
            super::super::access::Refusal::None => CollectionDisposition::Failed,
        },
    }
}

fn combined(error: CrawlError, persistence: CrawlError) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("{error}; retaining roster failure also failed: {persistence}"),
    }
}

fn empty(team: &TeamRef, run: &RosterRun<'_>) -> Journal {
    Journal {
        team_id: team.id.clone(),
        school: None,
        year: run.school_year.get(),
        observed_on: run.observed_on.to_string(),
        capture: None,
        refusal: None,
        quarantine: None,
        rejected: Vec::new(),
        unfinished: None,
        athletes: 0,
        teams: 0,
        disposition: CollectionDisposition::Unknown,
        co2027: 0,
        co2027_boys: 0,
        co2027_girls: 0,
    }
}
