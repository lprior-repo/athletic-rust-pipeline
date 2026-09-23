//! The `Athletes` sheet (objective §50): one row per canonical athlete in the run's cohort.
//!
//! Published columns, in this order: Athlete ID, Name, State, School, Graduation Year, Current Grade,
//! TF, XC, Indoor, Outdoor, Event list, Headline PR summary, Performance count, Meet count,
//! Athletic.net URL, MileSplit URL, Other profile URLs, Head TF Coach, Head XC Coach, Coach
//! Professional Email, Athletic Director, School Athletics URL, Public Recruiting GPA, GPA Source,
//! Sources Count, Identity Confidence, Coverage State, Conflict Flag, Review Flag, School City,
//! Head TF Coach Email, Head XC Coach Email, AD Email, Preferred Recruiting Contact,
//! Preferred Contact Role, Preferred Contact Email, Contact State.
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
//! `row_for` publishes those cells as the concatenation of its column groups — identity, the sport
//! flags with the event list, headline and counts, the profile URLs, the school's coach columns, the
//! athlete's audit columns, and the contact ladder's answer — so the published order is the order the
//! groups are listed in.
//!
//! `Public Recruiting GPA` and `GPA Source` carry no cell: `census-domain` has no GPA observation
//! entity, and objective §36 forbids inferring one, so the workbook states the absence rather than a
//! number nothing in the store supports.
//!
//! # Who to contact, and how the sheet says it
//!
//! The last eight columns answer the recruiter's question. `School City` is the city on the athlete's
//! school row (`CanonicalSchool::city`), blank when that row carries none. The three school-level
//! email columns are the published addresses of the coaches the sheet already names: `Head TF Coach
//! Email` is the address of the coach in `Head TF Coach`, `Head XC Coach Email` the address of
//! `Head XC Coach`, and `AD Email` the athletic director's — the same rows `Coaches` (§53) prints.
//!
//! `Preferred Recruiting Contact`, `Preferred Contact Role`, `Preferred Contact Email` and
//! `Contact Coverage State` are the athlete-specific answer, and the whole rule lives in [`super::contact`]:
//!
//! 1. the head coach of the athlete's evidence-bearing sport — a track slot (`Head TF Coach`) for an
//!    athlete with stored indoor or outdoor track evidence, a cross-country slot (`Head XC Coach`)
//!    for an athlete whose only sport is cross country, and the track slot for an athlete that stores
//!    no sport; `Preferred Contact Role` names the slot, plus the side of the team when the coach's
//!    row was published for one side (`Head TF Coach (girls)`);
//! 2. else the school's other head-coach slot, the same fallback `Coach Professional Email` takes;
//! 3. else a head coach whose row carries no sport binding;
//! 4. else the athletic director.
//!
//! `Contact Coverage State` is the typed `ContactState` vocabulary and is never blank: a coach's published
//! address is `professional_coach_email`, the director's is `professional_ad_email`, a named contact
//! with no published address anywhere is `coach_name_only`, a school whose coach rows name neither a
//! head coach nor a director is `no_public_contact_found`, and a school with no coach row at all is
//! `contact_source_not_attempted`. A blank cell therefore never means "we did not look".
//!
//! Where a school's rows name more than one head coach of one sport, the athlete's own side of the
//! team is preferred (the source publishes `Boys` and `Girls` head coaches for one sport), and within
//! one side the row that published an address wins, newest `Evidence.observed_on` first, then by
//! coach name and id — so two head coaches with different addresses never leave the cell to chance.
//! Only `CanonicalCoach::professional_email` is ever written to an email cell: an absent or withheld
//! mailbox stays blank, and no address is ever derived from a name or a domain.

use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, Confidence, Sport};

use super::super::cells::{cell, row, Cell};
use super::columns::{
    conflicts, coverage_state, current_grade, event_list, headline, source_count,
};
use super::contact::{self, Named, Preferred, SchoolContacts};
use super::dataset::Dataset;
use super::facts::AthleteTally;
use super::profiles::{profiles_of, Profiles};
use super::prs::PrRow;

/// The worksheet name, as objective §50 publishes it.
pub(super) const TITLE: &str = "Athletes";

/// The sheet's column headers, in published order.
pub(super) const HEADERS: [&str; 37] = [
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
    "School City",
    "Head TF Coach Email",
    "Head XC Coach Email",
    "AD Email",
    "Preferred Recruiting Contact",
    "Preferred Contact Role",
    "Preferred Contact Email",
    "Contact Coverage State",
];

/// Column widths, one per header.
pub(super) const WIDTHS: [u16; 37] = [
    20, 26, 8, 30, 10, 8, 5, 5, 7, 8, 40, 46, 10, 10, 30, 30, 40, 24, 24, 30, 24, 30, 8, 12, 8, 8,
    14, 8, 8, 20, 30, 30, 30, 26, 24, 30, 26,
];

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

/// One athlete's published row: the column groups below, each contributing its columns in published
/// order.
fn row_for(dataset: &Dataset, athlete: &CanonicalAthlete) -> ReportResult<Vec<Cell>> {
    let school = athlete.school.as_str();
    let tally = dataset.tallies.get(athlete.id.as_str());
    let prs: Vec<&PrRow> = dataset.prs_of(athlete.id.as_str()).collect();
    let profiles = profiles_of(athlete);
    let contacts = dataset.contacts.get(school);
    let preferred = contact::preferred(contacts, athlete);
    let director = contacts.and_then(|contacts| contacts.director.as_ref());
    let mut cells = identity_cells(dataset, athlete);
    cells.extend(participation_cells(athlete, tally, &prs)?);
    cells.extend(profile_cells(profiles));
    cells.extend(school_cells(dataset, athlete, contacts, director));
    cells.extend(audit_cells(dataset, athlete, tally, &prs)?);
    cells.extend(contact_cells(contacts, director, preferred));
    Ok(cells)
}

/// The identity columns: the athlete's stored id and canonical name, the school the run places them
/// at, and the cohort year with its newest observed grade.
fn identity_cells(dataset: &Dataset, athlete: &CanonicalAthlete) -> Vec<Cell> {
    let school = athlete.school.as_str();
    row!(
        Cell::text(athlete.id.as_str()),
        Cell::text(athlete.canonical_name.clone()),
        Cell::text(dataset.school_state(school)),
        Cell::text(dataset.school_name(school)),
        Cell::Number(f64::from(athlete.grad_year.get())),
        current_grade(athlete),
    )
}

/// The participation and best-mark columns: the four sport flags, the event kinds the athlete has a
/// stored mark in, the `PRs` sheet's own headline, and the athlete's performance and meet counts.
fn participation_cells(
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&PrRow],
) -> ReportResult<Vec<Cell>> {
    Ok(row!(
        flag(plays_track(athlete)),
        flag(athlete.sports.contains(&Sport::CrossCountry)),
        flag(athlete.sports.contains(&Sport::IndoorTrack)),
        flag(athlete.sports.contains(&Sport::OutdoorTrack)),
        Cell::text(event_list(tally)),
        Cell::text(headline(prs)),
        Cell::number(tally.map_or(0, |tally| tally.performances))?,
        Cell::number(tally.map_or(0, |tally| tally.meets.len()))?,
    ))
}

/// The three profile-URL columns, in the order [`Profiles`] splits them.
fn profile_cells(profiles: Profiles) -> Vec<Cell> {
    row!(
        published(profiles.athletic_net),
        published(profiles.milesplit),
        Cell::text(profiles.other.join("; ")),
    )
}

/// The school-level columns: the head coaches the sheet names, the address of each, the athletic
/// director, the school's athletics page, and the two GPA columns the objective states as an absence
/// rather than a number nothing in the store supports.
fn school_cells(
    dataset: &Dataset,
    athlete: &CanonicalAthlete,
    contacts: Option<&SchoolContacts>,
    director: Option<&Named>,
) -> Vec<Cell> {
    let school = athlete.school.as_str();
    row!(
        published(contacts.and_then(|contacts| contacts.head_track.clone())),
        published(contacts.and_then(|contacts| contacts.head_cross_country.clone())),
        published(contacts.and_then(|contacts| contacts.coach_email.clone())),
        published(director.map(|director| director.name.clone())),
        Cell::text(dataset.athletics_url(school)),
        Cell::Empty,
        Cell::Empty,
    )
}

/// The athlete's audit columns — sources count, identity confidence, coverage state, and the conflict
/// and review flags — then the school's city, the last school-level cell before the contact ladder.
fn audit_cells(
    dataset: &Dataset,
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&PrRow],
) -> ReportResult<Vec<Cell>> {
    let school = athlete.school.as_str();
    Ok(row!(
        Cell::number(source_count(athlete))?,
        Cell::Number(f64::from(athlete.identity_confidence.get())),
        Cell::text(coverage_state(
            tally.is_some_and(|tally| tally.performances > 0),
            !prs.is_empty()
        )),
        flag(conflicts(athlete)),
        flag(athlete.identity_confidence < Confidence::HIGH),
        Cell::text(dataset.school_city(school)),
    ))
}

/// The recruiter-facing contact columns: the school's published addresses, then [`contact`]'s answer
/// for this athlete — who to write to, in which role, at which address, and the typed state that says
/// whether the school published a contact at all.
fn contact_cells(
    contacts: Option<&SchoolContacts>,
    director: Option<&Named>,
    preferred: Preferred,
) -> Vec<Cell> {
    row!(
        published(contacts.and_then(|contacts| contacts.head_track_email.clone())),
        published(contacts.and_then(|contacts| contacts.head_cross_country_email.clone())),
        published(director.and_then(|director| director.email.clone())),
        Cell::text(preferred.name),
        Cell::text(preferred.role),
        Cell::text(preferred.email),
        Cell::text(preferred.state.as_str()),
    )
}

/// One column whose value the store may not hold: the stored value, or the blank cell the sheet
/// prints when it holds none. A blank is "not published", never a value derived from something else.
fn published(value: Option<String>) -> Cell {
    Cell::text(value.unwrap_or_default())
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
