//! The `Athletes` sheet (objective §50): one row per canonical athlete in the run's cohort.
//!
//! Published columns, in this order: Athlete ID, Name, State, School, Graduation Year, Current Grade,
//! TF, XC, Indoor, Outdoor, Event list, Headline PR summary, Performance count, Meet count,
//! Athletic.net URL, MileSplit URL, Other profile URLs, Head TF Coach, Head XC Coach, Coach
//! Professional Email, Athletic Director, School Athletics URL, Public Recruiting GPA, GPA Source,
//! Sources Count, Identity Confidence, Coverage State, Conflict Flag, Review Flag.
//!
//! Every cell is a stored field or a documented rule over stored fields:
//!
//! * the sport flags and the event list come from `CanonicalAthlete::sports` and the athlete's scoped
//!   performances' event kinds;
//! * `Current Grade` is the grade of the most recently observed `ObservedGrade`, which is evidence
//!   (§3), never a re-derivation of the cohort;
//! * the coach columns come from the school's head coaches and athletic director in the coach table;
//! * `Coverage State` is `pr` when the athlete has a row on the `PRs` sheet, `performance` when the
//!   athlete has stored performances but no comparable PR (relay legs and unparsed marks), and
//!   `identity-only` when the athlete has no scoped performance at all;
//! * `Conflict Flag` is `yes` when a grade observation implies a different graduating class than the
//!   athlete's canonical one, or when one source namespace carries two different ids for the athlete —
//!   the two disagreements the deterministic merge cannot settle on its own;
//! * `Review Flag` is `yes` when the athlete's identity confidence is below
//!   `Confidence::HIGH`, i.e. no grade observation agrees with the canonical cohort.
//!
//! `Public Recruiting GPA` and `GPA Source` carry no cell: `census-domain` has no GPA observation
//! entity, and objective §36 forbids inferring one, so the workbook states the absence rather than a
//! number nothing in the store supports.

use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, Confidence, SourceNamespace, Sport};
use std::collections::{BTreeMap, BTreeSet};

use super::super::cells::{cell, row, Cell};
use super::dataset::Dataset;
use super::facts::AthleteTally;
use super::prs::PrRow;

/// The worksheet name, as objective §50 publishes it.
pub(super) const TITLE: &str = "Athletes";

/// The sheet's column headers, in published order.
pub(super) const HEADERS: [&str; 29] = [
    "Athlete ID",
    "Name",
    "State",
    "School",
    "Graduation Year",
    "Current Grade",
    "TF",
    "XC",
    "Indoor",
    "Outdoor",
    "Event list",
    "Headline PR summary",
    "Performance count",
    "Meet count",
    "Athletic.net URL",
    "MileSplit URL",
    "Other profile URLs",
    "Head TF Coach",
    "Head XC Coach",
    "Coach Professional Email",
    "Athletic Director",
    "School Athletics URL",
    "Public Recruiting GPA",
    "GPA Source",
    "Sources Count",
    "Identity Confidence",
    "Coverage State",
    "Conflict Flag",
    "Review Flag",
];

/// Column widths, one per header.
pub(super) const WIDTHS: [u16; 29] = [
    20, 26, 8, 30, 10, 8, 5, 5, 7, 8, 40, 46, 10, 10, 30, 30, 40, 24, 24, 30, 24, 30, 8, 12, 8, 8,
    14, 8, 8,
];

/// How many of an athlete's PRs the headline column names.
const HEADLINE_PRS: usize = 5;

/// The `Athletes` sheet, ordered by state, school, then athlete.
pub(super) fn sheet(dataset: &Dataset) -> ReportResult<Vec<Vec<Cell>>> {
    let mut ordered: Vec<&CanonicalAthlete> = dataset.athletes.iter().collect();
    ordered.sort_by_key(|athlete| {
        (
            dataset.school_state(athlete.school.as_str()),
            dataset.school_name(athlete.school.as_str()),
            athlete.canonical_name.clone(),
        )
    });
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for athlete in ordered {
        rows.push(row_for(dataset, athlete)?);
    }
    Ok(rows)
}

/// One athlete's published row.
fn row_for(dataset: &Dataset, athlete: &CanonicalAthlete) -> ReportResult<Vec<Cell>> {
    let school = athlete.school.as_str();
    let tally = dataset.tallies.get(athlete.id.as_str());
    let prs: Vec<&PrRow> = dataset.prs_of(athlete.id.as_str()).collect();
    let profiles = profiles_of(athlete);
    let contacts = dataset.contacts.get(school);
    Ok(row!(
        Cell::text(athlete.id.as_str()),
        Cell::text(athlete.canonical_name.clone()),
        Cell::text(dataset.school_state(school)),
        Cell::text(dataset.school_name(school)),
        Cell::Number(f64::from(athlete.grad_year.get())),
        current_grade(athlete),
        flag(plays_track(athlete)),
        flag(athlete.sports.contains(&Sport::CrossCountry)),
        flag(athlete.sports.contains(&Sport::IndoorTrack)),
        flag(athlete.sports.contains(&Sport::OutdoorTrack)),
        Cell::text(event_list(tally)),
        Cell::text(headline(&prs)),
        Cell::number(tally.map_or(0, |tally| tally.performances))?,
        Cell::number(tally.map_or(0, |tally| tally.meets.len()))?,
        Cell::text(profiles.athletic_net.unwrap_or_default()),
        Cell::text(profiles.milesplit.unwrap_or_default()),
        Cell::text(profiles.other.join("; ")),
        Cell::text(
            contacts
                .and_then(|contacts| contacts.head_track.clone())
                .unwrap_or_default()
        ),
        Cell::text(
            contacts
                .and_then(|contacts| contacts.head_cross_country.clone())
                .unwrap_or_default()
        ),
        Cell::text(
            contacts
                .and_then(|contacts| contacts.coach_email.clone())
                .unwrap_or_default()
        ),
        Cell::text(
            contacts
                .and_then(|contacts| contacts.director.clone())
                .unwrap_or_default()
        ),
        Cell::text(dataset.athletics_url(school)),
        Cell::Empty,
        Cell::Empty,
        Cell::number(source_count(athlete))?,
        Cell::Number(f64::from(athlete.identity_confidence.0)),
        Cell::text(coverage_state(
            tally.is_some_and(|tally| tally.performances > 0),
            !prs.is_empty()
        )),
        flag(conflicts(athlete)),
        flag(athlete.identity_confidence < Confidence::HIGH),
    ))
}

/// `yes` for a recorded fact, blank otherwise; a blank is "the store does not say", never "no".
fn flag(recorded: bool) -> Cell {
    if recorded {
        Cell::text("yes")
    } else {
        Cell::Empty
    }
}

fn plays_track(athlete: &CanonicalAthlete) -> bool {
    athlete.sports.contains(&Sport::IndoorTrack) || athlete.sports.contains(&Sport::OutdoorTrack)
}

/// The grade of the most recently observed grade entry, newest school year first and then the higher
/// grade; blank when the cohort came from a source that published no grade.
fn current_grade(athlete: &CanonicalAthlete) -> Cell {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|observation| {
            (
                observation.school_year.start_year(),
                observation.grade.get(),
            )
        })
        .map(|observation| Cell::text(observation.grade.to_string()))
        .unwrap_or(Cell::Empty)
}

fn event_list(tally: Option<&AthleteTally>) -> String {
    tally
        .map(|tally| {
            tally
                .events
                .iter()
                .cloned()
                .collect::<Vec<String>>()
                .join("; ")
        })
        .unwrap_or_default()
}

/// The first [`HEADLINE_PRS`] PRs in the `PRs` sheet's own order for this athlete, so the headline
/// and the PR rows agree.
fn headline(prs: &[&PrRow]) -> String {
    prs.iter()
        .take(HEADLINE_PRS)
        .map(|pr| format!("{} {}", pr.event, pr.mark))
        .collect::<Vec<String>>()
        .join("; ")
}

/// How many distinct source namespaces the athlete is known through — the census's own
/// "multiple sources" rule.
fn source_count(athlete: &CanonicalAthlete) -> usize {
    athlete
        .source_identities
        .iter()
        .map(|identity| &identity.namespace)
        .collect::<BTreeSet<&SourceNamespace>>()
        .len()
}

fn conflicts(athlete: &CanonicalAthlete) -> bool {
    athlete
        .observed_grades
        .iter()
        .any(|observation| observation.grad_year() != athlete.grad_year)
        || conflicting_identities(athlete)
}

/// One source namespace holding two different external ids for this athlete: the merge kept both
/// observations and cannot decide which identity is right.
fn conflicting_identities(athlete: &CanonicalAthlete) -> bool {
    let mut seen: BTreeMap<&SourceNamespace, &str> = BTreeMap::new();
    for identity in &athlete.source_identities {
        match seen.get(&identity.namespace) {
            Some(existing) if *existing != identity.id.as_str() => return true,
            Some(_) => {}
            None => {
                seen.insert(&identity.namespace, identity.id.as_str());
            }
        }
    }
    false
}

fn coverage_state(has_performance: bool, has_pr: bool) -> &'static str {
    match (has_performance, has_pr) {
        (true, true) => "pr",
        (true, false) => "performance",
        (false, _) => "identity-only",
    }
}

/// The athlete's public profile URLs split into the sheet's three columns, de-duplicated in stored
/// order: Athletic.net and MileSplit by publication host, everything else in `Other profile URLs`.
#[derive(Default)]
struct Profiles {
    athletic_net: Option<String>,
    milesplit: Option<String>,
    other: Vec<String>,
}

fn profiles_of(athlete: &CanonicalAthlete) -> Profiles {
    let mut profiles = Profiles::default();
    let mut seen: Vec<String> = Vec::new();
    let candidates = athlete.public_profile_urls.iter().cloned().chain(
        athlete
            .source_identities
            .iter()
            .filter_map(|identity| identity.url.clone()),
    );
    for url in candidates {
        if url.is_empty() || seen.contains(&url) {
            continue;
        }
        seen.push(url.clone());
        place_url(&mut profiles, url);
    }
    profiles
}

/// File one URL under the column whose host it belongs to.
fn place_url(profiles: &mut Profiles, url: String) {
    let lowered = url.to_ascii_lowercase();
    if lowered.contains("athletic.net") {
        if profiles.athletic_net.is_none() {
            profiles.athletic_net = Some(url);
        }
    } else if lowered.contains("milesplit") {
        if profiles.milesplit.is_none() {
            profiles.milesplit = Some(url);
        }
    } else {
        profiles.other.push(url);
    }
}
