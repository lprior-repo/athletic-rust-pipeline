//! The parent tables a §52 row joins: read once under the requested scope, indexed by id.
//!
//! A performance names its athlete, meet and event, and the athlete names the school; those are the
//! four tables [`Parents`] reads. The reading applies exactly the filters the row assembly applies —
//! the core-source rule ([`Scope::Core`]) and the run-scope rule (ADR-009) — so a row can only cite a
//! parent the report itself would publish. [`Lookups`] then indexes each table by the id its rows
//! carry, which makes one performance four lookups rather than four scans.
//!
//! A parent the store does not hold is `None`. The row is still written: §52 asks for every stored
//! performance, not only the rows whose joins resolve, and [`Joins`] leaves the cells a missing parent
//! would have filled blank. `State` is the one absence with two spellings — a meet whose venue was
//! never placed prints [`MEET_STATE_UNRESOLVED`], while a meet row the store does not hold prints
//! nothing.

use super::rows::PerformanceRow;
use crate::bests::{mark_text, sport_of, Measure};
use crate::report::{
    exclude_out_of_scope, in_run_scope, jurisdiction_of, retain_core, school_state_index,
    ReportResult, Scope,
};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    Evidence, Mark, MEET_STATE_UNRESOLVED,
};
use census_domain::JurisdictionBucket;
use census_store::{Store, Table};
use std::collections::HashMap;

/// The parent tables a row joins, read under `scope` and the run scope, exactly as the report applies
/// both. Held owned so a [`Lookups`] can borrow all four at once.
pub(super) struct Parents {
    athletes: Vec<CanonicalAthlete>,
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
}

impl Parents {
    pub(super) fn read(store: &Store, scope: Scope) -> ReportResult<Self> {
        let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
        let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
        let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
        if scope == Scope::Core {
            retain_core(&mut athletes);
            retain_core(&mut meets);
            retain_core(&mut events);
        }
        // Run-scope filter: exclude jurisdictions outside CENSUS_SCOPE (ADR-009). Athlete
        // jurisdiction comes from school state (the report's rule), and the placement index carries
        // the excluded school rows too — so an athlete whose school the run scope leaves out is
        // excluded with it rather than read as unplaced and then cited with a raw school id.
        let mut schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut school_state = school_state_index(&schools);
        school_state.extend(school_state_index(&outside_schools));
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
        Ok(Self {
            athletes,
            schools,
            meets,
            events,
        })
    }

    pub(super) fn lookups(&self) -> Lookups<'_> {
        Lookups::of(&self.athletes, &self.schools, &self.meets, &self.events)
    }
}

/// The canonical rows a performance names, indexed by id: a row is four lookups, never a scan.
pub(super) struct Lookups<'a> {
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    events: HashMap<&'a str, &'a CanonicalEvent>,
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

    /// The school names a row can print, sorted and deduplicated: the bucket universe.
    pub(super) fn school_names(&self) -> Vec<&'a str> {
        let mut names: Vec<&'a str> = self
            .schools
            .values()
            .map(|school| school.name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// One joined row.
    pub(super) fn row(&self, performance: &CanonicalPerformance) -> PerformanceRow {
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

/// One performance's parents, `None` where the store holds no such row. The parents are borrowed
/// from the indexed tables (`'a`) and the cited observation from the performance itself (`'b`).
struct Joins<'a, 'b> {
    athlete: Option<&'a CanonicalAthlete>,
    school: Option<&'a CanonicalSchool>,
    meet: Option<&'a CanonicalMeet>,
    event: Option<&'a CanonicalEvent>,
    source: Option<&'b Evidence>,
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
            .map(|row| row.kind.stable_key().into_owned())
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
/// The comparable number on the mark's own scale, converted back to f64 for the wire form.
fn normalized_mark(mark: &Mark) -> Option<f64> {
    Measure::of(mark).and_then(|measure| measure.value(mark)).map(|v| v as f64 / 100.0)
}
