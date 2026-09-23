//! The §52 row assembly: one row per stored performance, joined to the canonical rows it names.
//!
//! A performance is stored as ids, so each row copies the athlete (name, graduation year), the
//! athlete's school, the meet (name, venue state) and the event (label, round) out of the canonical
//! rows [`Store::scan`] returns, all under the requested [`Scope`]. A parent row the store does not
//! hold leaves its cells empty and the performance is still written: "every available performance"
//! means every stored row, not only the rows whose joins resolve. `State` tells the two kinds of
//! absence apart: [`MEET_STATE_UNRESOLVED`] is the report's own spelling for a meet whose venue was
//! never placed, while a blank state means the meet row itself is absent.
//!
//! [`PerformanceRow::cells`] is the one place a cell is formatted. `Mark` is the source's own
//! notation (`10.94`, `4:41.23`, `5' 4"`), `Normalized Mark` is the same mark on its own comparable
//! scale (seconds, metres or points) and stays blank for a mark the census has not parsed yet,
//! `Sport` is the event family the `Best results` sheet already publishes, `Timing`, `Wind`, `Round`
//! and `Place` are the performance's own published conditions, and `Source`, `Source ResultID` and
//! `Source URL` are the observation the row rests on: the lexicographically first source that
//! observed it, the provider-local result key, and that source's URL.
//!
//! Relay legs are included. They are deliberately not personal bests (see [`crate::bests`]), but §52
//! asks for every performance, and a 4x400 leg is one.

use crate::bests::{mark_text, sport_of, Measure};
use crate::report::{in_run_scope, jurisdiction_of, retain_core, school_state_index, ReportResult, Scope};
use crate::store::{Store, Table};
use super::super::cells::{cell, row, Cell};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    Evidence, Mark, MEET_STATE_UNRESOLVED,
};
use census_domain::JurisdictionBucket;
use std::cmp::Ordering;
use std::collections::HashMap;

/// The §52 rows, in sheet order.
pub(super) fn performance_rows(store: &Store, scope: Scope) -> ReportResult<Vec<PerformanceRow>> {
    let mut performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
    if scope == Scope::Core {
        retain_core(&mut performances);
        retain_core(&mut athletes);
        retain_core(&mut meets);
        retain_core(&mut events);
    }
    // Run-scope filter: exclude jurisdictions outside CENSUS_SCOPE (ADR-009).
    // Athlete jurisdiction comes from school state (the report's rule).
    // Performances stay: the cohort join in accumulate() already drops out-of-cohort athletes.
    let mut schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    schools.retain(|s| in_run_scope(JurisdictionBucket::from(s.state)));
    let school_state = school_state_index(&schools);
    athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
    meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
    let lookups = Lookups::of(&athletes, &schools, &meets, &events);
    let mut rows: Vec<PerformanceRow> = performances
        .iter()
        .map(|performance| lookups.row(performance))
        .collect();
    rows.sort_by(sheet_order);
    Ok(rows)
}

/// The order the sheets publish and therefore the order they are cut into partitions: school, date,
/// athlete, event, then the unique performance id, which breaks every remaining tie.
fn sheet_order(left: &PerformanceRow, right: &PerformanceRow) -> Ordering {
    left.school
        .cmp(&right.school)
        .then_with(|| left.date.cmp(&right.date))
        .then_with(|| left.athlete.cmp(&right.athlete))
        .then_with(|| left.athlete_id.cmp(&right.athlete_id))
        .then_with(|| left.event.cmp(&right.event))
        .then_with(|| left.id.cmp(&right.id))
}

/// One §52 row, joined and formatted; the sheet writer only copies these fields out.
///
/// The fields are `pub(super)` so the module's tests can assert the join column by column; nothing
/// outside this module tree reads them.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PerformanceRow {
    pub(super) id: String,
    pub(super) athlete_id: String,
    pub(super) athlete: String,
    pub(super) school: String,
    pub(super) grad_year: Option<i16>,
    pub(super) meet_id: String,
    pub(super) meet: String,
    pub(super) date: String,
    pub(super) state: Option<String>,
    pub(super) sport: String,
    pub(super) event: String,
    pub(super) mark: String,
    pub(super) normalized: Option<f64>,
    pub(super) timing: Option<String>,
    pub(super) wind_mps: Option<f64>,
    pub(super) round: Option<String>,
    pub(super) place: Option<u16>,
    pub(super) source: String,
    pub(super) source_result: String,
    pub(super) source_url: String,
}

impl PerformanceRow {
    /// The row's cells, in [`COLUMNS`](super::COLUMNS) order.
    pub(super) fn cells(&self) -> Vec<Cell> {
        row!(
            Cell::text(self.athlete_id.clone()),
            Cell::text(self.athlete.clone()),
            Cell::text(self.school.clone()),
            self.grad_year
                .map(|year| Cell::Number(f64::from(year)))
                .unwrap_or(Cell::Empty),
            Cell::text(self.meet_id.clone()),
            Cell::text(self.meet.clone()),
            Cell::text(self.date.clone()),
            Cell::text(self.state.clone().unwrap_or_default()),
            Cell::text(self.sport.clone()),
            Cell::text(self.event.clone()),
            Cell::text(self.mark.clone()),
            self.normalized.map(Cell::Number).unwrap_or(Cell::Empty),
            Cell::text(self.timing.clone().unwrap_or_default()),
            self.wind_mps.map(Cell::Number).unwrap_or(Cell::Empty),
            Cell::text(self.round.clone().unwrap_or_default()),
            self.place
                .map(|place| Cell::Number(f64::from(place)))
                .unwrap_or(Cell::Empty),
            Cell::text(self.source.clone()),
            Cell::text(self.source_result.clone()),
            Cell::text(self.source_url.clone()),
        )
    }
}

/// The canonical rows a performance names, indexed by id: a row is four lookups, never a scan.
struct Lookups<'a> {
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    events: HashMap<&'a str, &'a CanonicalEvent>,
}

/// One performance's parents, `None` where the store holds no such row. The parents are borrowed
/// from the indexed tables (`'a`) and the cited observation from the performance itself (`'b`).
struct Joins<'a, 'b> {
    athlete: Option<&'a CanonicalAthlete>,
    school: Option<&'a CanonicalSchool>,
    meet: Option<&'a CanonicalMeet>,
    event: Option<&'a CanonicalEvent>,
    source: Option<&'b Evidence>,
}

impl<'a> Lookups<'a> {
    /// Index the parent tables by the id each row carries.
    fn of(
        athletes: &'a [CanonicalAthlete],
        schools: &'a [CanonicalSchool],
        meets: &'a [CanonicalMeet],
        events: &'a [CanonicalEvent],
    ) -> Self {
        Self {
            athletes: index(athletes, |row| row.id.as_str()),
            schools: index(schools, |row| row.id.as_str()),
            meets: index(meets, |row| row.id.as_str()),
            events: index(events, |row| row.id.as_str()),
        }
    }

    /// One joined row.
    fn row(&self, performance: &CanonicalPerformance) -> PerformanceRow {
        let joins = self.joins(performance);
        PerformanceRow {
            id: performance.id.as_str().to_string(),
            athlete_id: performance.athlete.as_str().to_string(),
            athlete: joins.athlete_name(),
            school: joins.school_name(),
            grad_year: joins.grad_year(),
            meet_id: performance.meet.as_str().to_string(),
            meet: joins.meet_name(),
            date: performance.date.clone(),
            state: joins.state(),
            sport: joins.sport(),
            event: joins.event_label(),
            mark: mark_text(&performance.mark),
            normalized: normalized_mark(&performance.mark),
            timing: performance.timing.map(|method| format!("{method:?}")),
            wind_mps: performance.wind_mps,
            round: joins.round_of(performance),
            place: performance.place,
            source: joins.source_id(),
            source_result: performance.source_key.clone(),
            source_url: joins.source_url(),
        }
    }

    /// The rows `performance` names, plus the observation that names its source.
    fn joins<'b>(&self, performance: &'b CanonicalPerformance) -> Joins<'a, 'b> {
        let athlete = self.athletes.get(performance.athlete.as_str()).copied();
        Joins {
            athlete,
            school: athlete.and_then(|row| self.schools.get(row.school.as_str()).copied()),
            meet: self.meets.get(performance.meet.as_str()).copied(),
            event: self.events.get(performance.event.as_str()).copied(),
            source: observed(performance),
        }
    }
}

impl Joins<'_, '_> {
    /// The athlete's canonical name, empty when the athlete row is absent.
    fn athlete_name(&self) -> String {
        self.athlete
            .map(|row| row.canonical_name.clone())
            .unwrap_or_default()
    }

    /// The athlete's school name, empty when either row is absent.
    fn school_name(&self) -> String {
        self.school.map(|row| row.name.clone()).unwrap_or_default()
    }

    /// The athlete's graduation year, blank when the athlete row is absent.
    fn grad_year(&self) -> Option<i16> {
        self.athlete.map(|row| row.grad_year.get())
    }

    /// The meet's name, or the venue's state: `??` for a venue never placed, blank for a meet row the
    /// store does not hold at all.
    fn state(&self) -> Option<String> {
        self.meet.map(|row| {
            row.state.map_or_else(
                || MEET_STATE_UNRESOLVED.to_string(),
                |state| state.code().to_string(),
            )
        })
    }

    fn meet_name(&self) -> String {
        self.meet.map(|row| row.name.clone()).unwrap_or_default()
    }

    /// The event's family, in the vocabulary the `Best results` sheet already publishes.
    fn sport(&self) -> String {
        self.event
            .map(|row| sport_of(&row.kind).to_string())
            .unwrap_or_default()
    }

    /// The event as the census names it (`Track1600m`, `LongJump`, …), blank for an absent event.
    fn event_label(&self) -> String {
        self.event
            .map(|row| format!("{:?}", row.kind))
            .unwrap_or_default()
    }

    /// The round the performance was run in, falling back to the round its event was published in.
    fn round_of(&self, performance: &CanonicalPerformance) -> Option<String> {
        performance
            .round
            .clone()
            .or_else(|| self.event.and_then(|row| row.round.clone()))
    }

    fn source_id(&self) -> String {
        self.source
            .map(|row| row.source.id.clone())
            .unwrap_or_default()
    }

    fn source_url(&self) -> String {
        self.source
            .and_then(|row| row.source.url.clone())
            .unwrap_or_default()
    }
}

/// Index a parent table by the id each row carries.
fn index<'a, T>(rows: &'a [T], id: impl Fn(&'a T) -> &'a str) -> HashMap<&'a str, &'a T> {
    rows.iter().map(|row| (id(row), row)).collect()
}

/// The observation a row cites: the lexicographically first source id, ties broken by URL, so the
/// choice never depends on the order the observations were merged in.
fn observed(performance: &CanonicalPerformance) -> Option<&Evidence> {
    performance.evidence.iter().min_by(|left, right| {
        left.source
            .id
            .cmp(&right.source.id)
            .then_with(|| left.source.url.cmp(&right.source.url))
    })
}

/// The comparable number on the mark's own scale. A [`Mark::Raw`] value has no scale yet, so it stays
/// blank instead of being invented.
fn normalized_mark(mark: &Mark) -> Option<f64> {
    Measure::of(mark).and_then(|measure| measure.value(mark))
}
