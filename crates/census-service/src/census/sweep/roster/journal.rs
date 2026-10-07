use census_crawl::milesplit::{
    self, Roster, RosterOutcome, RosterQuarantine, RosterRejection, RosterVerdict, Site, TeamRef,
};
use census_crawl::net::FetchOutcome;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Gender, SchoolYear,
    SourceAthleteObservation, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::{StoreBatch, Table};
use serde::{Deserialize, Serialize};

use crate::census::scope::{count_co2027, count_cohort};

#[derive(Serialize)]
pub(super) struct Records {
    school: CanonicalSchool,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    school_observation: SourceObservation,
    athlete_observations: Vec<SourceObservation>,
}

impl Records {
    pub(super) fn of(
        roster: &Roster,
        site: &Site,
        year: SchoolYear,
        observed_on: &str,
    ) -> CrawlResult<Self> {
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

    pub(super) fn stage(&self, batch: &mut StoreBatch<'_>) -> CrawlResult<()> {
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
pub(super) struct Journal<S = String, C = FetchOutcome, R = Vec<RosterRejection>> {
    pub(super) team_id: S,
    pub(super) school: Option<S>,
    pub(super) year: i16,
    pub(super) observed_on: S,
    pub(super) capture: Option<C>,
    pub(super) refusal: Option<S>,
    pub(super) quarantine: Option<RosterQuarantine>,
    pub(super) rejected: R,
    pub(super) athletes: usize,
    pub(super) teams: usize,
    pub(super) co2027: usize,
    pub(super) co2027_boys: usize,
    pub(super) co2027_girls: usize,
}

pub(super) fn quarantine_of(verdict: &RosterVerdict) -> Option<RosterQuarantine> {
    match verdict {
        RosterVerdict::Quarantined { reason, .. } => Some(*reason),
        RosterVerdict::Complete { .. } | RosterVerdict::Partial { .. } => None,
    }
}

pub(super) fn roster_journal<'a>(
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
        capture: Some(&read.capture),
        refusal: None,
        quarantine: quarantine_of(&read.verdict),
        rejected: read.verdict.rejections(),
        athletes: roster.map_or(0, |rows| rows.athletes.len()),
        teams: records.map_or(0, |rows| rows.teams.len()),
        co2027: roster.map_or(0, count_co2027),
        co2027_boys: roster.map_or(0, |rows| count_cohort(rows, Gender::Boys)),
        co2027_girls: roster.map_or(0, |rows| count_cohort(rows, Gender::Girls)),
    }
}

pub(super) fn roster_digest(
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

pub(super) fn roster_operation(
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
