//! The `PRs` sheet (objective §51): one row per athlete and event.
//!
//! The mark is the platform's own best-mark rule — the reduction `bests` emits in its text sidecars —
//! carried with the recruiter columns §51 asks for: the winning performance's result URL, how many
//! sources attest the event, and which marks two stored rows for the same meet and event disagree on.
//! Relay legs are excluded for the same reason `bests` excludes them: a squad mark is not a personal
//! best, and an unparsed `Mark::Raw` has no comparable value.
//!
//! Published columns, in this order: Athlete ID, Athlete, Gender, School, State, Graduation Year,
//! Sport, Event, Season, Calculated PR, Mark Value, Unit, Wind, PR date, Meet, Place, Result URL,
//! Source count, PR conflict.
//!
//! `Calculated PR` remains the source's own notation. `Mark Value` and `Unit` are the shared numeric
//! rendering of that same winning mark, so readers can sort and filter it without reparsing text.

use crate::bests::{is_relay, mark_text, mark_unit, mark_value, sport_of, Measure};
use crate::report::ReportResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, EventKind, Mark, Sport,
};
use census_domain::JurisdictionBucket;
use std::collections::{BTreeMap, BTreeSet};

use super::super::cells::{row, Cell};

/// The worksheet name, as objective §51 publishes it.
pub(super) const TITLE: &str = "PRs";

/// The sheet's column headers, in published order.
pub(super) const HEADERS: [&str; 19] = [
    "Athlete ID",
    "Athlete",
    "Gender",
    "School",
    "State",
    "Graduation Year",
    "Sport",
    "Event",
    "Season",
    "Calculated PR",
    "Mark Value",
    "Unit",
    "Wind",
    "PR date",
    "Meet",
    "Place",
    "Result URL",
    "Source count",
    "PR conflict",
];

/// Column widths, one per header.
pub(super) const WIDTHS: [u16; 19] = [
    20, 26, 10, 30, 8, 16, 12, 20, 14, 18, 14, 10, 12, 14, 40, 10, 44, 13, 30,
];

#[derive(Debug, Clone)]
pub(super) struct PrRow {
    pub(super) athlete_id: String,
    pub(super) athlete: String,
    pub(super) gender: String,
    pub(super) school: String,
    pub(super) state: JurisdictionBucket,
    pub(super) grad_year: i16,
    pub(super) source_mark: Mark,
    pub(super) sport: String,
    pub(super) event: String,
    pub(super) season: String,
    pub(super) mark: String,
    pub(super) mark_value: Option<f64>,
    pub(super) unit: Option<&'static str>,
    pub(super) wind_mps: Option<f64>,
    pub(super) date: String,
    pub(super) meet: String,
    pub(super) place: Option<u16>,
    pub(super) result_url: String,
    /// How many distinct sources attest this event for the athlete.
    pub(super) source_count: usize,
    pub(super) disagreements: Vec<String>,
}

pub(super) struct SchoolFacts {
    pub(super) name: String,
    pub(super) state: JurisdictionBucket,
}

#[derive(Default)]
struct Slot {
    winner: Option<(PrRow, i32, Measure)>,
    sources: BTreeSet<String>,
    reports: BTreeMap<String, BTreeSet<String>>,
}

struct Context<'a> {
    athlete: &'a CanonicalAthlete,
    kind: &'a EventKind,
    meet: Option<&'a CanonicalMeet>,
    school: Option<&'a SchoolFacts>,
}

pub(super) fn reduce(
    athletes: &[CanonicalAthlete],
    performances: &[CanonicalPerformance],
    kinds: &BTreeMap<String, EventKind>,
    meets: &BTreeMap<String, CanonicalMeet>,
    schools: &BTreeMap<String, SchoolFacts>,
) -> Vec<PrRow> {
    let cohort: BTreeMap<&str, &CanonicalAthlete> = athletes
        .iter()
        .map(|athlete| (athlete.id.as_str(), athlete))
        .collect();
    let mut slots: BTreeMap<(String, String), Slot> = BTreeMap::new();
    for performance in performances {
        let Some(athlete) = cohort.get(performance.athlete.as_str()).copied() else {
            continue;
        };
        let Some(kind) = kinds.get(performance.event.as_str()) else {
            continue;
        };
        if is_relay(kind) {
            continue;
        }
        fold(
            &mut slots,
            performance,
            &Context {
                athlete,
                kind,
                meet: meets.get(performance.meet.as_str()),
                school: schools.get(athlete.school.as_str()),
            },
        );
    }
    into_rows(slots, meets)
}

fn fold(
    slots: &mut BTreeMap<(String, String), Slot>,
    performance: &CanonicalPerformance,
    context: &Context<'_>,
) {
    let Some(measure) = Measure::of(&performance.mark) else {
        return;
    };
    let Some(value) = measure.value(&performance.mark) else {
        return;
    };
    let key = (
        context.athlete.id.as_str().to_string(),
        context.kind.stable_key().into_owned(),
    );
    let slot = slots.entry(key).or_default();
    for evidence in &performance.evidence {
        slot.sources.insert(evidence.source.id.clone());
    }
    slot.reports
        .entry(performance.meet.as_str().to_string())
        .or_default()
        .insert(mark_text(&performance.mark));
    let wins = slot
        .winner
        .as_ref()
        .map(|(_, incumbent, _)| measure.better(value, *incumbent))
        .unwrap_or(true);
    if wins {
        slot.winner = Some((row_for(context, performance), value, measure));
    }
}

fn row_for(context: &Context<'_>, performance: &CanonicalPerformance) -> PrRow {
    let school = context.school;
    PrRow {
        athlete_id: context.athlete.id.as_str().to_string(),
        athlete: context.athlete.canonical_name.clone(),
        gender: context.athlete.gender.stable_key().to_string(),
        school: school
            .map(|school| school.name.clone())
            .unwrap_or_else(|| context.athlete.school.as_str().to_string()),
        state: school
            .map(|school| school.state)
            .unwrap_or(JurisdictionBucket::Unplaced),
        grad_year: context.athlete.grad_year.get(),
        sport: sport_of(context.kind).to_string(),
        event: context.kind.stable_key().into_owned(),
        season: season_of(context.meet),
        mark: mark_text(&performance.mark),
        mark_value: mark_value(&performance.mark),
        source_mark: performance.mark.clone(),
        unit: mark_unit(&performance.mark),
        wind_mps: performance.wind_mps,
        date: performance.date.clone(),
        meet: context
            .meet
            .map(|meet| meet.name.clone())
            .unwrap_or_default(),
        place: performance.place,
        result_url: result_url(performance),
        source_count: 0,
        disagreements: Vec::new(),
    }
}

fn result_url(performance: &CanonicalPerformance) -> String {
    performance
        .evidence
        .iter()
        .find_map(|evidence| evidence.source.url.clone())
        .unwrap_or_default()
}

fn season_of(meet: Option<&CanonicalMeet>) -> String {
    let Some(meet) = meet else {
        return String::new();
    };
    let indoor = meet.sports.contains(&Sport::IndoorTrack);
    let outdoor = meet.sports.contains(&Sport::OutdoorTrack);
    match (indoor, outdoor) {
        (true, true) => "Indoor; Outdoor".to_string(),
        (true, false) => "Indoor".to_string(),
        (false, true) => "Outdoor".to_string(),
        (false, false) => String::new(),
    }
}

fn into_rows(
    slots: BTreeMap<(String, String), Slot>,
    meets: &BTreeMap<String, CanonicalMeet>,
) -> Vec<PrRow> {
    let mut rows: Vec<PrRow> = slots
        .into_values()
        .filter_map(|slot| close(slot, meets))
        .collect();
    rows.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| left.athlete.cmp(&right.athlete))
    });
    rows
}

fn close(slot: Slot, meets: &BTreeMap<String, CanonicalMeet>) -> Option<PrRow> {
    let (mut row, _, _) = slot.winner?;
    row.source_count = slot.sources.len();
    row.disagreements = slot
        .reports
        .iter()
        .filter(|(_, marks)| marks.len() > 1)
        .map(|(meet, marks)| disagreement(meet, marks, meets))
        .collect();
    Some(row)
}

fn disagreement(
    meet: &str,
    marks: &BTreeSet<String>,
    meets: &BTreeMap<String, CanonicalMeet>,
) -> String {
    let name = meets.get(meet).map_or(meet, |row| row.name.as_str());
    let published: Vec<&str> = marks.iter().map(String::as_str).collect();
    format!("{name}: {}", published.join(" | "))
}

pub(super) fn sheet(prs: &[PrRow]) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for pr in prs {
        rows.push(row!(
            Cell::text(pr.athlete_id.clone()),
            Cell::text(pr.athlete.clone()),
            Cell::text(pr.gender.clone()),
            Cell::text(pr.school.clone()),
            Cell::text(pr.state.code()),
            Cell::Number(f64::from(pr.grad_year)),
            Cell::text(pr.sport.clone()),
            Cell::text(pr.event.clone()),
            Cell::text(pr.season.clone()),
            Cell::text(pr.mark.clone()),
            pr.mark_value.map_or(Cell::Empty, Cell::Number),
            pr.unit.map_or(Cell::Empty, Cell::text),
            pr.wind_mps.map_or(Cell::Empty, Cell::Number),
            Cell::text(pr.date.clone()),
            Cell::text(pr.meet.clone()),
            pr.place
                .map_or(Cell::Empty, |place| Cell::Number(f64::from(place))),
            Cell::text(pr.result_url.clone()),
            Cell::number(pr.source_count)?,
            if pr.disagreements.is_empty() {
                Cell::Empty
            } else {
                Cell::text(pr.disagreements.join("; "))
            },
        ));
    }
    Ok(rows)
}
