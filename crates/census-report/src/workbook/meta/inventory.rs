//! The two row inventories: every canonical school and every canonical meet the store retains.
//!
//! Both sheets are one row per stored row — never a summary — because they are the two tables the
//! operator cross-checks a provider against: a school with no coach and no athlete is visible as
//! exactly that, and a meet no source placed in a jurisdiction prints `??` rather than a guess. The
//! meet sheet is also the meet-first acquisition view (§24): a meet whose `Source identities` name a
//! provider namespace is one the adapters can request by id instead of enumerating.
//!
//! The per-school athlete and coach tallies are computed here, in one pass over the two tables, so
//! the sheet never re-scans and never prints a number the census would not.

use crate::report::ReportResult;
use census_domain::model::{
    CanonicalMeet, CanonicalSchool, CompetitionLevel, GradYear, SourceIdentity, Sport,
    MEET_STATE_UNRESOLVED,
};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

use crate::workbook::cells::{cell, row, Cell};

use super::{bump, StoreRows};

/// Widths for the school inventory.
pub(super) const SCHOOL_WIDTHS: [u16; 13] = [14, 32, 8, 16, 12, 11, 8, 15, 10, 14, 14, 28, 34];

/// Widths for the meet inventory.
pub(super) const MEET_WIDTHS: [u16; 10] = [14, 44, 12, 12, 8, 13, 22, 24, 40, 28];

/// One row per canonical school, annotated with the athletes and coaches the store attaches to it.
pub(super) fn schools_sheet(rows: &StoreRows) -> ReportResult<Vec<Vec<Cell>>> {
    let facts = SchoolFacts::of(rows);
    let mut cells = vec![row!(
        "School ID",
        "School",
        "State",
        "City",
        "Association",
        "Enrollment",
        "Co-op",
        "Class of 2027",
        "Athletes",
        "Track/XC coaches",
        "Coaches with email",
        "Athletics site",
        "Source identities",
    )];
    let mut sorted: Vec<&CanonicalSchool> = rows.schools.iter().collect();
    sorted.sort_by(|left, right| {
        state_label(left.state)
            .cmp(state_label(right.state))
            .then_with(|| left.name.cmp(&right.name))
    });
    for school in sorted {
        let id = school.id.as_str();
        cells.push(row!(
            Cell::text(id),
            Cell::text(school.name.clone()),
            Cell::text(state_label(school.state)),
            Cell::text(school.city.clone().unwrap_or_default()),
            Cell::text(school.association.clone().unwrap_or_default()),
            enrollment_cell(school.enrollment),
            Cell::text(if school.co_op { "co-op" } else { "" }),
            Cell::number(SchoolFacts::count(&facts.class_of_2027, id))?,
            Cell::number(SchoolFacts::count(&facts.athletes, id))?,
            Cell::number(SchoolFacts::count(&facts.coaches, id))?,
            Cell::number(SchoolFacts::count(&facts.coaches_with_email, id))?,
            Cell::text(school.athletics_website.clone().unwrap_or_default()),
            Cell::text(identities_text(&school.source_identities)),
        ));
    }
    Ok(cells)
}

/// One row per canonical meet, by date then name: the meet-first acquisition view (§24), where the
/// source identities decide whether the meet can be requested by id rather than enumerated.
pub(super) fn meets_sheet(meets: &[CanonicalMeet]) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!(
        "Meet ID",
        "Meet",
        "Date",
        "End date",
        "State",
        "Level",
        "Sports",
        "Location",
        "Source identities",
        "Source URLs",
    )];
    let mut sorted: Vec<&CanonicalMeet> = meets.iter().collect();
    sorted.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.as_str().cmp(right.id.as_str()))
    });
    for meet in sorted {
        cells.push(row!(
            Cell::text(meet.id.as_str()),
            Cell::text(meet.name.clone()),
            Cell::text(meet.date.clone()),
            Cell::text(meet.end_date.clone().unwrap_or_default()),
            Cell::text(
                meet.state
                    .map_or(MEET_STATE_UNRESOLVED, UsJurisdiction::code)
            ),
            Cell::text(level_label(meet.level)),
            Cell::text(sport_list(&meet.sports)),
            Cell::text(meet.location.clone().unwrap_or_default()),
            Cell::text(identities_text(&meet.source_identities)),
            Cell::text(meet.source_urls.first().cloned().unwrap_or_default()),
        ));
    }
    cells
}

/// Per-school tallies the school sheet annotates its rows with: one pass each over the athlete and
/// coach tables, so the sheet itself never re-scans.
#[derive(Default)]
struct SchoolFacts<'a> {
    athletes: BTreeMap<&'a str, usize>,
    class_of_2027: BTreeMap<&'a str, usize>,
    coaches: BTreeMap<&'a str, usize>,
    coaches_with_email: BTreeMap<&'a str, usize>,
}

impl<'a> SchoolFacts<'a> {
    fn of(rows: &'a StoreRows) -> Self {
        let mut facts = Self::default();
        for athlete in &rows.athletes {
            bump(facts.athletes.entry(athlete.school.as_str()).or_default());
            if athlete.grad_year == GradYear::CO2027 {
                bump(
                    facts
                        .class_of_2027
                        .entry(athlete.school.as_str())
                        .or_default(),
                );
            }
        }
        for coach in &rows.coaches {
            bump(facts.coaches.entry(coach.school.as_str()).or_default());
            if coach.professional_email.is_some() {
                bump(
                    facts
                        .coaches_with_email
                        .entry(coach.school.as_str())
                        .or_default(),
                );
            }
        }
        facts
    }

    /// One school's tally: a school no row names counts zero, never missing.
    fn count(counts: &BTreeMap<&str, usize>, school: &str) -> usize {
        counts.get(school).copied().unwrap_or(0)
    }
}

/// A jurisdiction as the sheet prints it: its USPS code, or `UNKNOWN` for a row the store could not
/// place.
fn state_label(state: Option<UsJurisdiction>) -> &'static str {
    state.map_or("UNKNOWN", UsJurisdiction::code)
}

/// A school's enrollment: an unpublished one stays blank rather than reading as zero.
fn enrollment_cell(enrollment: Option<u32>) -> Cell {
    match enrollment {
        Some(value) => Cell::Number(f64::from(value)),
        None => Cell::Empty,
    }
}

/// A competition level as the sheet prints it.
fn level_label(level: CompetitionLevel) -> String {
    format!("{level:?}").to_lowercase()
}

/// The sports one row covers, in the short vocabulary the adapters bucket their own rows with.
fn sport_list(sports: &[Sport]) -> String {
    sports
        .iter()
        .map(|sport| sport_label(*sport))
        .collect::<Vec<&str>>()
        .join(", ")
}

/// One sport's sheet label.
fn sport_label(sport: Sport) -> &'static str {
    match sport {
        Sport::CrossCountry => "xc",
        Sport::IndoorTrack => "indoor",
        Sport::OutdoorTrack => "outdoor",
    }
}

/// A row's source identities as `count (namespaces)`: the namespaces are what a reader compares
/// across rows, and the count says how much evidence sits behind them.
fn identities_text(identities: &[SourceIdentity]) -> String {
    if identities.is_empty() {
        return String::new();
    }
    let mut namespaces: Vec<String> = identities
        .iter()
        .map(|identity| identity.namespace.to_string())
        .collect();
    namespaces.sort();
    namespaces.dedup();
    format!("{} ({})", identities.len(), namespaces.join(", "))
}
