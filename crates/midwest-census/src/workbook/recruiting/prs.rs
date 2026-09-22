//! The `PRs` sheet (objective §51): one row per athlete and event.
//!
//! The mark is the platform's own best-mark rule — the reduction `bests` publishes on the `Best
//! results` sheet — carried with the recruiter columns §51 asks for: the winning performance's
//! result URL, how many sources attest the event, and whether two stored rows for the same meet and
//! event disagree on the mark. Relay legs are excluded for the same reason `bests` excludes them: a
//! squad mark is not a personal best, and an unparsed `Mark::Raw` has no comparable value.
//!
//! Published columns, in this order: Athlete ID, Athlete, School, State, Sport, Event,
//! Indoor/Outdoor, Calculated PR, PR date, Meet, Result URL, Athletic.net reported PR, MileSplit
//! reported PR, Other reported PR, Source count, PR conflict.
//!
//! Three columns carry no cell because the store holds no entity behind them: `census-domain` models
//! no per-source *reported* best, only the performances a source published, so §51's reported-PR
//! columns stay blank instead of restating the calculated PR as if a source had claimed it.

use crate::bests::{is_relay, mark_text, sport_of, Measure};
use crate::report::ReportResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, EventKind, Sport,
};
use std::collections::{BTreeMap, BTreeSet};

use super::super::cells::{cell, row, Cell};

/// The worksheet name, as objective §51 publishes it.
pub(super) const TITLE: &str = "PRs";

/// The sheet's column headers, in published order.
pub(super) const HEADERS: [&str; 16] = [
    "Athlete ID",
    "Athlete",
    "School",
    "State",
    "Sport",
    "Event",
    "Indoor/Outdoor",
    "Calculated PR",
    "PR date",
    "Meet",
    "Result URL",
    "Athletic.net reported PR",
    "MileSplit reported PR",
    "Other reported PR",
    "Source count",
    "PR conflict",
];

/// Column widths, one per header.
pub(super) const WIDTHS: [u16; 16] = [
    20, 26, 30, 8, 12, 16, 14, 14, 12, 40, 44, 12, 12, 12, 13, 12,
];

/// One published PR row.
#[derive(Debug, Clone)]
pub(super) struct PrRow {
    pub(super) athlete_id: String,
    pub(super) athlete: String,
    pub(super) school: String,
    /// The athlete's school jurisdiction, or `UNKNOWN` when no school row places the school.
    pub(super) state: String,
    pub(super) sport: String,
    pub(super) event: String,
    /// `Indoor`, `Outdoor`, both, or blank when the meet records neither.
    pub(super) season: String,
    /// The winning mark in the source's own published notation.
    pub(super) mark: String,
    pub(super) date: String,
    pub(super) meet: String,
    /// The winning performance's evidence URL, blank when the source published none.
    pub(super) result_url: String,
    /// Distinct evidence sources across every stored mark of this athlete in this event.
    pub(super) source_count: usize,
    /// Two stored rows for the same meet and event publish different marks.
    pub(super) conflict: bool,
}

/// The school facts a PR row prints, resolved by the caller from the school table.
pub(super) struct SchoolFacts {
    pub(super) name: String,
    pub(super) state: String,
}

/// One `(athlete, event)` reduction slot: the mark that currently wins, the sources that attest the
/// event, and every mark text seen per meet.
#[derive(Default)]
struct Slot {
    winner: Option<(PrRow, f64, Measure)>,
    sources: BTreeSet<String>,
    reports: BTreeMap<String, BTreeSet<String>>,
}

/// The athlete, event kind, meet and school one performance is folded under.
struct Context<'a> {
    athlete: &'a CanonicalAthlete,
    kind: &'a EventKind,
    meet: Option<&'a CanonicalMeet>,
    school: Option<&'a SchoolFacts>,
}

/// Reduce the scoped performances to one row per athlete and event.
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
    into_rows(slots)
}

/// Fold one performance into its slot, keeping the mark that wins on its own scale.
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
        format!("{:?}", context.kind),
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

/// The published row for one winning mark, before its source count and conflict flag are known.
fn row_for(context: &Context<'_>, performance: &CanonicalPerformance) -> PrRow {
    let school = context.school;
    PrRow {
        athlete_id: context.athlete.id.as_str().to_string(),
        athlete: context.athlete.canonical_name.clone(),
        school: school
            .map(|school| school.name.clone())
            .unwrap_or_else(|| context.athlete.school.as_str().to_string()),
        state: school
            .map(|school| school.state.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        sport: sport_of(context.kind).to_string(),
        event: format!("{:?}", context.kind),
        season: season_of(context.meet),
        mark: mark_text(&performance.mark),
        date: performance.date.clone(),
        meet: context
            .meet
            .map(|meet| meet.name.clone())
            .unwrap_or_default(),
        result_url: result_url(performance),
        source_count: 0,
        conflict: false,
    }
}

/// The winning performance's evidence URL: the first stored evidence entry that carries one, so the
/// row cites the document the mark was read from.
fn result_url(performance: &CanonicalPerformance) -> String {
    performance
        .evidence
        .iter()
        .find_map(|evidence| evidence.source.url.clone())
        .unwrap_or_default()
}

/// Which track season a meet was recorded in, from the meet's own sport list.
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

/// Close every slot into a published row, then order the sheet: state, event, athlete.
fn into_rows(slots: BTreeMap<(String, String), Slot>) -> Vec<PrRow> {
    let mut rows: Vec<PrRow> = slots.into_values().filter_map(close).collect();
    rows.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| left.athlete.cmp(&right.athlete))
    });
    rows
}

/// One slot's winning row, finished: how many sources attest the event, and whether any meet carries
/// two different marks for it.
fn close(slot: Slot) -> Option<PrRow> {
    let (mut row, _, _) = slot.winner?;
    row.source_count = slot.sources.len();
    row.conflict = slot.reports.values().any(|marks| marks.len() > 1);
    Some(row)
}

/// The `PRs` sheet: the frozen header, then one row per athlete and event.
pub(super) fn sheet(prs: &[PrRow]) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for pr in prs {
        rows.push(row!(
            Cell::text(pr.athlete_id.clone()),
            Cell::text(pr.athlete.clone()),
            Cell::text(pr.school.clone()),
            Cell::text(pr.state.clone()),
            Cell::text(pr.sport.clone()),
            Cell::text(pr.event.clone()),
            Cell::text(pr.season.clone()),
            Cell::text(pr.mark.clone()),
            Cell::text(pr.date.clone()),
            Cell::text(pr.meet.clone()),
            Cell::text(pr.result_url.clone()),
            Cell::Empty,
            Cell::Empty,
            Cell::Empty,
            Cell::number(pr.source_count)?,
            if pr.conflict {
                Cell::text("yes")
            } else {
                Cell::Empty
            },
        ));
    }
    Ok(rows)
}
