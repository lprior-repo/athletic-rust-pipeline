use census_crawl::milesplit::{
    self, RosterOutcome, RosterQuarantine, RosterRejection, RosterVerdict, Site, TeamRef,
};
use census_crawl::net::FetchOutcome;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalSchool, CanonicalTeam, SchoolYear,
    SourceAthleteObservation, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::{Store, StoreBatch, Table};
use serde::{Deserialize, Serialize};

use super::super::units::RosterRun;
use super::{counts::Counts, metadata::Window};
use sha2::{Digest, Sha256};

pub(super) struct Records {
    school: CanonicalSchool,
    teams: Vec<CanonicalTeam>,
    athletes: Vec<CanonicalAthlete>,
    school_observation: SourceObservation,
    athlete_observations: Vec<SourceObservation>,
}

impl Records {
    pub(super) fn of(window: &Window<'_>, site: &Site, year: SchoolYear) -> CrawlResult<Self> {
        let (school, athletes, teams) = milesplit::roster_entities(
            window.team(),
            window.athletes(),
            year,
            window.observed_on(),
            site,
        )?;
        let school_observation = school_observation(&school, window.observed_on())?;
        let athlete_observations = athlete_observations(&athletes, &school, window.observed_on())?;
        Ok(Self {
            school,
            teams,
            athletes,
            school_observation,
            athlete_observations,
        })
    }

    pub(super) fn stage(&self, store: &Store, batch: &mut StoreBatch<'_>) -> CrawlResult<String> {
        let mut digest = Sha256::new();
        super::effects::stage(
            store,
            batch,
            Table::Schools,
            std::slice::from_ref(&self.school),
            &mut digest,
        )?;
        super::effects::stage(store, batch, Table::Teams, &self.teams, &mut digest)?;
        super::effects::stage(store, batch, Table::Athletes, &self.athletes, &mut digest)?;
        super::effects::stage(
            store,
            batch,
            Table::SourceObservations,
            std::slice::from_ref(&self.school_observation),
            &mut digest,
        )?;
        super::effects::stage(
            store,
            batch,
            Table::SourceObservations,
            &self.athlete_observations,
            &mut digest,
        )?;
        Ok(format!("{:x}", digest.finalize()))
    }

    pub(super) fn teams(&self) -> &[CanonicalTeam] {
        &self.teams
    }
}

fn school_observation(school: &CanonicalSchool, at: &str) -> CrawlResult<SourceObservation> {
    SourceSchoolObservation::of_school(&SourceNamespace::MilesplitSchool, school, at)
        .map(SourceObservation::School)
        .ok_or_else(|| CrawlError::Invariant {
            detail: "roster school lacks its provider identity".to_string(),
        })
}

fn athlete_observations(
    athletes: &[CanonicalAthlete],
    school: &CanonicalSchool,
    at: &str,
) -> CrawlResult<Vec<SourceObservation>> {
    let mut rows = Vec::new();
    super::effects::reserve(&mut rows, athletes.len())?;
    athletes.iter().try_fold(rows, |mut rows, athlete| {
        let observation = SourceAthleteObservation::of_athlete(
            &SourceNamespace::MilesplitAthlete,
            athlete,
            Some(super::metadata::school_name(&school.name)?),
            at,
        )
        .ok_or_else(|| CrawlError::Invariant {
            detail: "roster athlete lacks its provider identity".to_string(),
        })?;
        rows.push(SourceObservation::Athlete(observation));
        Ok(rows)
    })
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Journal<S = String, C = FetchOutcome, R = Vec<RosterRejection>> {
    pub(super) team_id: S,
    pub(super) school: Option<S>,
    pub(super) year: i16,
    pub(super) observed_on: S,
    pub(super) capture: Option<C>,
    pub(super) refusal: Option<S>,
    pub(super) quarantine: Option<RosterQuarantine>,
    pub(super) rejected: R,
    #[serde(default)]
    pub(super) unfinished: Option<milesplit::SourceRowLocator>,
    #[serde(default)]
    pub(super) disposition: census_crawl::CollectionDisposition,
    pub(super) athletes: usize,
    pub(super) teams: usize,
    pub(super) co2027: usize,
    pub(super) co2027_boys: usize,
    pub(super) co2027_girls: usize,
}

impl Journal {
    pub(crate) fn team_id(&self) -> &str {
        &self.team_id
    }

    pub(crate) fn is_terminal_for(&self, team: &str, year: SchoolYear) -> bool {
        self.matches_scope(team, year) && self.is_terminal()
    }

    pub(crate) fn accepted_athletes(&self) -> usize {
        self.athletes
    }

    pub(crate) fn matches_scope(&self, team: &str, year: SchoolYear) -> bool {
        self.team_id == team && self.year == year.get()
    }

    pub(crate) fn disposition(&self) -> census_crawl::CollectionDisposition {
        self.disposition
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.disposition.is_complete()
            && self.capture.is_some()
            && self.refusal.is_none()
            && self.quarantine.is_none()
            && self.rejected.is_empty()
            && self.unfinished.is_none()
    }
}

pub(super) fn quarantine_of(verdict: &RosterVerdict) -> Option<RosterQuarantine> {
    match verdict {
        RosterVerdict::Quarantined { reason, .. } => Some(*reason),
        RosterVerdict::Complete { .. } | RosterVerdict::Partial { .. } => None,
    }
}

fn disposition_of(verdict: &RosterVerdict) -> census_crawl::CollectionDisposition {
    use census_crawl::CollectionDisposition;
    match verdict {
        RosterVerdict::Complete { .. } => CollectionDisposition::Complete,
        RosterVerdict::Partial { .. } => CollectionDisposition::Partial,
        RosterVerdict::Quarantined { .. } => CollectionDisposition::Quarantined,
    }
}

pub(super) fn roster_journal<'a>(
    team: &'a TeamRef,
    read: &'a RosterOutcome,
    records: Option<&'a Records>,
    run: &'a RosterRun<'_>,
    counts: &Counts,
) -> Journal<&'a str, &'a FetchOutcome, &'a [RosterRejection]> {
    Journal {
        team_id: team.id.as_str(),
        school: records.map(|rows| rows.school.id.as_str()),
        year: run.school_year.get(),
        observed_on: run.observed_on,
        capture: Some(&read.capture),
        refusal: None,
        quarantine: quarantine_of(&read.verdict),
        rejected: read.verdict.rejections(),
        unfinished: read.verdict.unfinished(),
        disposition: disposition_of(&read.verdict),
        athletes: counts.athletes,
        teams: counts.teams(),
        co2027: counts.co2027,
        co2027_boys: counts.boys,
        co2027_girls: counts.girls,
    }
}

pub(super) fn roster_digest(
    facts: &str,
    team: &TeamRef,
    run: &RosterRun<'_>,
    read: &RosterOutcome,
    offset: usize,
) -> CrawlResult<String> {
    serialized_digest(&(
        facts,
        offset,
        team.id.as_str(),
        run.school_year,
        run.observed_on,
        read.capture.url.as_str(),
        read.capture.status,
        read.capture.content_digest.as_str(),
        quarantine_of(&read.verdict),
        disposition_of(&read.verdict),
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
