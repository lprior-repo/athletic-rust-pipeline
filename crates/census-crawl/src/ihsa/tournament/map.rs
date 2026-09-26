//! Canonical mapping: the entities one tournament run's decoded rows become.
//!
//! The published ids are the point of this source. A finisher's `athlete.athleticNetId` and
//! `athleticLiveId` become [`SourceIdentity`] rows under the `AthleticNet` namespace — the same id
//! space the `athleticnet` adapter reads for itself — the team's `athleticNetId` becomes the team's,
//! and every row's `ihsaSchoolId` resolves the school. Nothing is invented: a row that publishes no
//! mark is not stored as a performance, and a row whose grade the payload leaves out mints no athlete,
//! because an athlete's identity keys on its graduating class.
//!
//! [`SourceIdentity`]: census_domain::model::SourceIdentity

use super::schools::Schools;
use crate::{AdapterContext, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, EventKind, Evidence, Gender, Grade, Mark, SchoolId, SchoolYear, SourceRef,
    Sport,
};
use std::collections::HashMap;

/// The association slug the `ihsa` schools adapter mints its school identities under.
pub(super) use crate::ihsa::ASSOCIATION;
/// The level string the sibling adapters put on a team.
pub(super) const HIGH_SCHOOL: &str = "high_school";

/// What every row of one run shares: the adapter its evidence is filed under and the observation
/// date stamped into it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Origin<'a> {
    pub(super) adapter: &'a str,
    pub(super) observed_on: &'a str,
}

impl Origin<'_> {
    /// The source reference a row's evidence names; `url` is the document it was read from.
    pub(super) fn source(&self, url: &str) -> SourceRef {
        SourceRef::new(self.adapter, Some(url.to_string()))
    }

    /// One parsed observation of the document at `url`.
    pub(super) fn evidence(&self, url: &str) -> Evidence {
        Evidence::parsed(self.source(url), self.observed_on)
    }

    /// One derived observation: the document publishes the row, the note says what was inferred.
    pub(super) fn derived(&self, url: &str, note: String) -> Evidence {
        Evidence::derived(self.source(url), self.observed_on, note)
    }
}

/// The canonical rows one run accumulates, keyed so a repeated entity is minted once.
#[derive(Default)]
pub(super) struct Accumulator {
    pub(super) schools: HashMap<String, CanonicalSchool>,
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
}

/// What one run saw and refused, in the order the report reads it.
#[derive(Debug, Default, Clone)]
pub(super) struct Stats {
    /// Finisher rows read from the event summaries.
    pub(super) rows: u64,
    /// Performance rows stored.
    pub(super) performances: u64,
    /// Relay legs minted as athletes: a relay row publishes the team's mark, not a leg's.
    pub(super) legs: u64,
    /// Rows dropped for a field the payload does not publish on that row.
    pub(super) rows_no_mark: u64,
    pub(super) rows_no_grade: u64,
    pub(super) rows_no_school: u64,
    /// Qualifier rows read from the cross-country lists, and the rows among them that carried a
    /// published grade (an athlete is keyed on its graduating class, so only those mint one).
    pub(super) qualifier_rows: u64,
    pub(super) qualifier_graded: u64,
    pub(super) qualifier_no_grade: u64,
    /// Event rows the walk minted, and the ones that publish no results.
    pub(super) events: u64,
    pub(super) events_without_results: u64,
    /// Schools minted because neither the association id nor the published name resolved.
    pub(super) schools_minted: u64,
}

/// One run's mapping sinks: the accumulator, the school resolver, the source and the counters.
pub(super) struct Mapper<'a> {
    pub(super) accumulated: Accumulator,
    pub(super) schools: Schools,
    pub(super) stats: Stats,
    pub(super) origin: Origin<'a>,
}

impl<'a> Mapper<'a> {
    /// Open a run's mapping state over the schools the store already holds.
    pub(super) fn load(
        ctx: &AdapterContext<'_>,
        adapter: &'a str,
        observed_on: &'a str,
    ) -> CrawlResult<Self> {
        Ok(Self {
            accumulated: Accumulator::default(),
            schools: Schools::load(ctx)?,
            stats: Stats::default(),
            origin: Origin {
                adapter,
                observed_on,
            },
        })
    }

    /// The run's counters, with the school resolver's tally folded in.
    pub(super) fn stats(&self) -> Stats {
        let mut stats = self.stats.clone();
        stats.schools_minted = self.schools.minted();
        stats
    }

    /// Count one event row the walk minted, and whether it publishes results.
    pub(super) fn count_event(&mut self, has_results: bool) {
        self.stats.events = self.stats.events.saturating_add(1);
        if !has_results {
            self.stats.events_without_results = self.stats.events_without_results.saturating_add(1);
        }
    }

    /// Resolve the school one finisher row names, counting a row that names none.
    pub(super) fn school(
        &mut self,
        ihsa_id: Option<&str>,
        name: Option<&str>,
        url: &str,
    ) -> Option<SchoolId> {
        let resolved = self
            .schools
            .resolve(ihsa_id, name, url, self.origin, &mut self.accumulated);
        if resolved.is_none() {
            self.stats.rows_no_school = self.stats.rows_no_school.saturating_add(1);
        }
        resolved
    }
}

/// The event one summary's rows are mapped against, and the season they belong to.
pub(super) struct EventContext<'a> {
    pub(super) meet: &'a CanonicalMeet,
    pub(super) event: &'a CanonicalEvent,
    pub(super) sport: Sport,
    /// The date this event was contested on: its own published date, else the meet's.
    pub(super) date: &'a str,
    pub(super) school_year: SchoolYear,
}

/// The cross-country state-final list one qualifier row was read from.
///
/// The list's class (`"1A"`) is not a field: nothing this adapter mints is keyed or classified by it,
/// and the report spells it from the request itself.
pub(super) struct XcList<'a> {
    pub(super) tournament_id: &'a str,
    pub(super) gender: Gender,
    pub(super) school_year: SchoolYear,
}

/// One athlete row's published facts.
pub(super) struct AthleteRow<'a> {
    pub(super) name: Option<&'a str>,
    pub(super) grade: Option<Grade>,
    pub(super) gender: Gender,
    pub(super) school: &'a SchoolId,
    pub(super) sport: Sport,
    pub(super) school_year: SchoolYear,
    pub(super) net_id: Option<u64>,
    pub(super) live_id: Option<u64>,
    /// The association's own athlete key, when the payload publishes one (XC entry numbers).
    pub(super) entry: Option<String>,
    pub(super) source_key: String,
}

/// One performance row's published facts.
pub(super) struct PerformanceRow<'a> {
    pub(super) date: &'a str,
    pub(super) mark: Mark,
    pub(super) place: Option<u16>,
    pub(super) grade: Option<Grade>,
    /// Provider-local result key, used for idempotent upserts.
    pub(super) source_key: String,
}

/// The published event name an ontology miss is filed under.
pub(super) fn unmapped(event_name: &str) -> EventKind {
    EventKind::Unmapped {
        label: event_name.trim().to_string(),
    }
}

/// One published gender token as the domain's.
///
/// The T&F surfaces publish `"M"`/`"F"`; a token the adapter does not know becomes
/// [`Gender::Unknown`] rather than a dropped event, so the row is still filed and the report shows
/// what the payload actually said.
pub(super) fn published_gender(code: &str) -> Gender {
    match code.trim() {
        "M" | "m" | "Male" | "Boys" | "boys" => Gender::Boys,
        "F" | "f" | "Female" | "Girls" | "girls" => Gender::Girls,
        "Mixed" | "mixed" | "Coed" | "coed" => Gender::Mixed,
        _ => Gender::Unknown,
    }
}

/// The athletes' published names, or `None` when the row publishes neither part.
pub(super) fn joined_name(first: Option<&str>, last: Option<&str>) -> Option<String> {
    let joined = format!("{} {}", first.unwrap_or_default(), last.unwrap_or_default());
    let trimmed = joined.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// The school year a published date falls in, on the domain's Aug 1 boundary.
///
/// A date the adapter cannot read — or one that names a year no season may open in — falls back to
/// the run's own school year rather than to a guess.
pub(super) fn school_year_of(date: &str, fallback: SchoolYear) -> SchoolYear {
    let year = date.get(..4).and_then(|part| part.parse::<i16>().ok());
    let month = date.get(5..7).and_then(|part| part.parse::<u8>().ok());
    match (year, month) {
        (Some(year), Some(month)) if (1..=12).contains(&month) => {
            SchoolYear::containing(year, month).unwrap_or(fallback)
        }
        _ => fallback,
    }
}

/// The school year a published term names (`"2025-26"` -> the year that opened in August 2025).
pub(super) fn school_year_of_term(term: &str) -> Option<SchoolYear> {
    let start = term.get(..4)?.parse::<i16>().ok()?;
    SchoolYear::new(start)
}
