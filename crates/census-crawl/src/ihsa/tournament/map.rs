use super::attestation::CaptureLineage;
use super::schools::Schools;
use crate::{AdapterContext, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, EventKind, Evidence, Gender, Grade, Mark, RelayResult, SchoolId, SchoolYear,
    SourceRef, Sport,
};
use std::collections::HashMap;

pub(super) use crate::ihsa::ASSOCIATION;
pub(super) const HIGH_SCHOOL: &str = "high_school";

#[derive(Debug, Clone)]
pub(super) struct Origin<'a> {
    pub(super) adapter: &'a str,
    pub(super) observed_on: String,
}

impl Origin<'_> {
    pub(super) fn source(&self, url: &str) -> SourceRef {
        SourceRef::new(self.adapter, Some(url.to_string()))
    }

    pub(super) fn evidence(&self, url: &str) -> Evidence {
        Evidence::parsed(self.source(url), &self.observed_on)
    }

    pub(super) fn derived(&self, url: &str, note: String) -> Evidence {
        Evidence::derived(self.source(url), &self.observed_on, note)
    }
}

#[derive(Default)]
pub(super) struct Accumulator {
    pub(super) schools: HashMap<String, CanonicalSchool>,
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
    pub(super) relay_results: HashMap<String, RelayResult>,
    pub(super) unsupported: crate::cohort::UnsupportedCohortRows,
}

#[derive(Debug, Default, Clone)]
pub(super) struct Stats {
    pub(super) rows: u64,
    pub(super) performances: u64,
    pub(super) legs: u64,
    pub(super) rows_no_mark: u64,
    pub(super) rows_no_grade: u64,
    pub(super) rows_no_school: u64,
    pub(super) qualifier_rows: u64,
    pub(super) qualifier_graded: u64,
    pub(super) qualifier_no_grade: u64,
    pub(super) events: u64,
    pub(super) events_without_results: u64,
    pub(super) schools_minted: u64,
}

pub(super) struct Mapper<'a> {
    pub(super) accumulated: Accumulator,
    pub(super) schools: Schools,
    pub(super) stats: Stats,
    pub(super) origin: Origin<'a>,
    pub(super) capture: Option<CaptureLineage>,
}

impl<'a> Mapper<'a> {
    pub(super) fn load(ctx: &AdapterContext<'_>, adapter: &'a str) -> CrawlResult<Self> {
        Ok(Self {
            accumulated: Accumulator::default(),
            capture: None,
            schools: Schools::load(ctx)?,
            stats: Stats::default(),
            origin: Origin {
                adapter,
                observed_on: String::new(),
            },
        })
    }

    pub(super) fn bind_capture(&mut self, capture: crate::net::FetchOutcome) -> CrawlResult<()> {
        let lineage = CaptureLineage::from_capture(capture)?;
        self.origin.observed_on = lineage.acquired_at().to_string();
        self.capture = Some(lineage);
        Ok(())
    }

    pub(super) fn stats(&self) -> Stats {
        let mut stats = self.stats.clone();
        stats.schools_minted = self.schools.minted();
        stats
    }

    pub(super) fn count_event(&mut self, has_results: bool) {
        self.stats.events = self.stats.events.saturating_add(1);
        if !has_results {
            self.stats.events_without_results = self.stats.events_without_results.saturating_add(1);
        }
    }

    pub(super) fn school(
        &mut self,
        ihsa_id: Option<&str>,
        name: Option<&str>,
        url: &str,
    ) -> Option<SchoolId> {
        let resolved =
            self.schools
                .resolve((ihsa_id, name, url), &self.origin, &mut self.accumulated);
        if resolved.is_none() {
            self.stats.rows_no_school = self.stats.rows_no_school.saturating_add(1);
        }
        resolved
    }
}

pub(super) struct EventContext<'a> {
    pub(super) meet: &'a CanonicalMeet,
    pub(super) event: &'a CanonicalEvent,
    pub(super) sport: Sport,
    pub(super) date: &'a str,
    pub(super) school_year: SchoolYear,
    pub(super) performance_as_of: chrono::NaiveDate,
}

pub(super) struct XcList<'a> {
    pub(super) tournament_id: &'a str,
    pub(super) gender: Gender,
    pub(super) school_year: SchoolYear,
}

pub(super) struct AthleteRow<'a> {
    pub(super) name: Option<&'a str>,
    pub(super) grade: Option<Grade>,
    pub(super) gender: Gender,
    pub(super) school: &'a SchoolId,
    pub(super) sport: Sport,
    pub(super) school_year: SchoolYear,
    pub(super) net_id: Option<u64>,
    pub(super) live_id: Option<u64>,
    pub(super) entry: Option<String>,
    pub(super) source_key: String,
}

pub(super) struct PerformanceRow<'a> {
    pub(super) date: &'a str,
    pub(super) mark: Mark,
    pub(super) place: Option<u16>,
    pub(super) grade: Option<Grade>,
    pub(super) source_key: String,
}

pub(super) fn unmapped(event_name: &str) -> EventKind {
    EventKind::Unmapped {
        label: event_name.trim().to_string(),
    }
}

pub(super) fn published_gender(code: &str) -> Gender {
    match code.trim() {
        "M" | "m" | "Male" | "Boys" | "boys" => Gender::Boys,
        "F" | "f" | "Female" | "Girls" | "girls" => Gender::Girls,
        "Mixed" | "mixed" | "Coed" | "coed" => Gender::Mixed,
        _ => Gender::Unknown,
    }
}

pub(super) fn joined_name(first: Option<&str>, last: Option<&str>) -> Option<String> {
    let joined = format!(
        "{} {}",
        first.map_or(Default::default(), core::convert::identity),
        last.map_or(Default::default(), core::convert::identity)
    );
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(super) fn school_year_of(date: &str, fallback: SchoolYear) -> SchoolYear {
    let year = date.get(..4).and_then(|part| part.parse::<i16>().ok());
    let month = date.get(5..7).and_then(|part| part.parse::<u8>().ok());
    match (year, month) {
        (Some(year), Some(month)) if (1..=12).contains(&month) => {
            SchoolYear::containing(year, month).map_or(fallback, |value| value)
        }
        _ => fallback,
    }
}

pub(super) fn school_year_of_term(term: &str) -> Option<SchoolYear> {
    let start = term.get(..4)?.parse::<i16>().ok()?;
    SchoolYear::new(start)
}
