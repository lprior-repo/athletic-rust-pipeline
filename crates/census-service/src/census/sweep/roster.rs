use census_crawl::milesplit::{
    self, Roster, RosterOutcome, RosterQuarantine, RosterRejection, RosterVerdict, Site, TeamRef,
};
use census_crawl::net::{FetchOptions, FetchOutcome, Fetcher};
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Gender, SchoolYear,
    SourceAthleteObservation, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::{Application, Store, StoreBatch, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::super::scope::{count_co2027, count_cohort};
use super::{rosters_phase, units::RosterRun};

#[derive(Serialize)]
struct Records {
    school: CanonicalSchool,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    school_observation: SourceObservation,
    athlete_observations: Vec<SourceObservation>,
}

impl Records {
    fn of(roster: &Roster, site: &Site, year: SchoolYear, observed_on: &str) -> CrawlResult<Self> {
        let (school, athletes, teams) = milesplit::roster_entities(roster, year, observed_on, site);
        let school_observation = SourceSchoolObservation::of_school(
            &SourceNamespace::MilesplitSchool,
            &school,
            observed_on,
        )
        .ok_or_else(|| CrawlError::Invariant {
            detail: format!(
                "roster {} produced a school without its provider identity",
                roster.team.id
            ),
        })?;
        let athlete_observations = athletes
            .iter()
            .map(|athlete| {
                SourceAthleteObservation::of_athlete(
                    &SourceNamespace::MilesplitAthlete,
                    athlete,
                    Some(school.name.clone()),
                    observed_on,
                )
                .map(SourceObservation::Athlete)
                .ok_or_else(|| CrawlError::Invariant {
                    detail: format!(
                        "roster {} produced an athlete without its provider identity",
                        roster.team.id
                    ),
                })
            })
            .collect::<CrawlResult<Vec<_>>>()?;
        Ok(Self {
            school,
            teams,
            athletes,
            school_observation: SourceObservation::School(school_observation),
            athlete_observations,
        })
    }

    fn stage(&self, batch: &mut StoreBatch<'_>) -> CrawlResult<()> {
        batch.append_many(Table::Schools, std::slice::from_ref(&self.school))?;
        batch.append_many(Table::Teams, &self.teams)?;
        batch.append_many(Table::Athletes, &self.athletes)?;
        batch.append_many(
            Table::SourceObservations,
            std::slice::from_ref(&self.school_observation),
        )?;
        batch.append_many(Table::SourceObservations, &self.athlete_observations)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Journal<S = String, C = FetchOutcome, R = Vec<RosterRejection>> {
    team_id: S,
    school: Option<S>,
    year: i16,
    observed_on: S,
    capture: C,
    quarantine: Option<RosterQuarantine>,
    rejected: R,
    athletes: usize,
    teams: usize,
    co2027: usize,
    co2027_boys: usize,
    co2027_girls: usize,
}

pub(super) async fn fetch_and_store(
    fetcher: &Fetcher,
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
) -> CrawlResult<Application> {
    let read = milesplit::fetch_roster(
        fetcher,
        team,
        &FetchOptions {
            refresh: run.refresh,
            allow_not_found: true,
            headers: Vec::new(),
        },
    )
    .await?;
    let roster = read.verdict.roster();
    let records = roster
        .map(|roster| Records::of(roster, &run.site, run.school_year, run.observed_on))
        .transpose()?;
    let quarantine = quarantine_of(&read.verdict);
    let journal = roster_journal(
        team,
        &read,
        records.as_ref(),
        run.school_year,
        run.observed_on,
        roster,
    );
    let digest = roster_digest(
        &records,
        team,
        run.school_year,
        run.observed_on,
        &read,
        quarantine,
    )?;
    let state = run.site.jurisdiction().code();
    let operation = roster_operation(state, run.school_year, run.revision, team);
    let phase = rosters_phase(run.site.jurisdiction(), run.school_year, run.revision);
    let mut batch = store.write_batch();
    if let Some(records) = &records {
        records.stage(&mut batch)?;
    }
    batch.journal_done(&phase, &format!("{state}:{}", team.id), &journal)?;
    Ok(batch.commit_once(&operation, &digest)?)
}

fn quarantine_of(verdict: &RosterVerdict) -> Option<RosterQuarantine> {
    match verdict {
        RosterVerdict::Quarantined { reason, .. } => Some(*reason),
        RosterVerdict::Complete { .. } | RosterVerdict::Partial { .. } => None,
    }
}

fn roster_journal<'a>(
    team: &'a TeamRef,
    read: &'a RosterOutcome,
    records: Option<&'a Records>,
    school_year: SchoolYear,
    observed_on: &'a str,
    roster: Option<&Roster>,
) -> Journal<&'a str, &'a FetchOutcome, &'a [RosterRejection]> {
    Journal {
        team_id: team.id.as_str(),
        school: records.map(|rows| rows.school.id.as_str()),
        year: school_year.get(),
        observed_on,
        capture: &read.capture,
        quarantine: quarantine_of(&read.verdict),
        rejected: read.verdict.rejections(),
        athletes: roster.map_or(0, |rows| rows.athletes.len()),
        teams: records.map_or(0, |rows| rows.teams.len()),
        co2027: roster.map_or(0, count_co2027),
        co2027_boys: roster.map_or(0, |rows| count_cohort(rows, Gender::Boys)),
        co2027_girls: roster.map_or(0, |rows| count_cohort(rows, Gender::Girls)),
    }
}

fn roster_digest(
    records: &Option<Records>,
    team: &TeamRef,
    school_year: SchoolYear,
    observed_on: &str,
    read: &RosterOutcome,
    quarantine: Option<RosterQuarantine>,
) -> CrawlResult<String> {
    serialized_digest(&(
        records,
        team.id.as_str(),
        school_year,
        observed_on,
        read.capture.url.as_str(),
        read.capture.status,
        read.capture.content_digest.as_str(),
        quarantine,
        read.verdict.rejections(),
    ))
    .map_err(|source| CrawlError::Canonical {
        table: "milesplit_roster".to_string(),
        source,
    })
}

fn roster_operation(
    state: &str,
    school_year: SchoolYear,
    revision: std::num::NonZeroU32,
    team: &TeamRef,
) -> String {
    format!(
        "milesplit_roster:{state}:{}:{revision}:{}",
        school_year.get(),
        team.id
    )
}

#[derive(Default)]
pub(super) struct Summary {
    pub(super) committed: usize,
    pub(super) athletes: usize,
    pub(super) co2027: usize,
    pub(super) co2027_boys: usize,
    pub(super) co2027_girls: usize,
    pub(super) errors: Vec<String>,
}

pub(super) fn summarize(store: &Store, phase: &str, teams: &[TeamRef]) -> CrawlResult<Summary> {
    let wanted: HashSet<&str> = teams.iter().map(|team| team.id.as_str()).collect();
    store.journal_payloads(phase)?.into_iter().try_fold(
        Summary::default(),
        |mut summary, payload| {
            let row: Journal =
                serde_json::from_value(payload).map_err(|source| CrawlError::Decode {
                    url: format!("journal:{phase}"),
                    source,
                })?;
            if !wanted.contains(row.team_id.as_str()) {
                return Ok(summary);
            }
            if row.quarantine.is_none() && row.rejected.is_empty() {
                summary.committed = sum(summary.committed, 1)?;
            } else {
                summary.errors.push(format!(
                    "{}: quarantine={:?}; rejected_rows={}",
                    row.capture.url,
                    row.quarantine,
                    row.rejected.len()
                ));
            }
            summary.athletes = sum(summary.athletes, row.athletes)?;
            summary.co2027 = sum(summary.co2027, row.co2027)?;
            summary.co2027_boys = sum(summary.co2027_boys, row.co2027_boys)?;
            summary.co2027_girls = sum(summary.co2027_girls, row.co2027_girls)?;
            Ok(summary)
        },
    )
}

fn sum(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "roster summary count overflowed".to_string(),
        })
}
