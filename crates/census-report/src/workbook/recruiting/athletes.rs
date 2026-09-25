//! The `Athletes` sheet (objective §50): one row per canonical athlete in the run's cohort.
//!
//! Published columns, in this order: Athlete ID, Name, Gender, Graduation Year, Current Grade, State,
//! School, School City, the three sport flags, the nineteen supported PR columns, performance and
//! meet counts, the school contact ladder, profile URLs, and the audit columns.
//!
//! Every cell is a stored field or a documented rule over stored fields:
//!
//! * the sport flags come from `CanonicalAthlete::sports`, and each PR column carries the winning
//!   mark from the same reduction the `PRs` sheet publishes;
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
//! `row_for` publishes those cells as identity, participation and PR columns, school contacts,
//! the preferred-contact ladder, profile URLs, and audit columns, in the order listed by `HEADERS`.
//!
//! Every published field is stored or a documented rule over stored fields. GPA columns are absent
//! because `census-domain` has no GPA observation entity and objective §36 forbids inferring one.
//!
//! # Who to contact, and how the sheet says it
//!
//! `School City` is the city on the athlete's school row (`CanonicalSchool::city`), blank when that row
//! carries none. The school contact columns carry the named coach/director rows and their own
//! professional-or-personal addresses, followed by the all-address inventory.
//!
//! `Preferred Recruiting Contact`, `Preferred Contact Role`, `Preferred Contact Email` and
//! `Contact Coverage State` are the athlete-specific answer, and the whole rule lives in [`super::contact`]:
//! The ladder reads `professional_email` first and explicitly falls back to `personal_email` when the
//! professional field is absent; the coverage state says a published address was found either way.
//! 1. the head coach of the athlete's evidence-bearing sport — a track slot (`Head TF Coach`) for an
//!    athlete with stored indoor or outdoor track evidence, a cross-country slot (`Head XC Coach`)
//!    for an athlete whose only sport is cross country, and the track slot for an athlete that stores
//!    no sport; `Preferred Contact Role` names the slot, plus the side of the team when the coach's
//!    row was published for one side (`Head TF Coach (girls)`);
//! 2. else the school's other head-coach slot;
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
//! An email cell carries the address its field holds, professional or personal; a blank means no source
//! published one. The `All Emails (school)` cell carries every coach-row address for that school.

use super::super::cells::{row, Cell};
use super::columns::{conflicts, coverage_state, current_grade, flag, published, source_count};
use super::contact::{self, Named, Preferred, SchoolContacts};
use super::dataset::Dataset;
use super::facts::AthleteTally;
use super::profiles::{profiles_of, Profiles};
use super::prs::PrRow;
use crate::bests::mark_value;
use crate::report::ReportResult;
use census_domain::model::{CanonicalAthlete, Confidence, Sport};

/// The worksheet name, as objective §50 publishes it.
pub(super) const TITLE: &str = "Athletes";

/// The sheet's column headers, in published order.
pub(super) const HEADERS: [&str; 51] = [
    "Athlete ID",
    "Name",
    "Gender",
    "Graduation Year",
    "Current Grade",
    "State",
    "School",
    "School City",
    "XC",
    "Indoor",
    "Outdoor",
    "100m (s)",
    "200m (s)",
    "400m (s)",
    "800m (s)",
    "1600m (s)",
    "3200m (s)",
    "1 Mile (s)",
    "5000m (s)",
    "100m Hurdles (s)",
    "110m Hurdles (s)",
    "300m Hurdles (s)",
    "High Jump (m)",
    "Long Jump (m)",
    "Triple Jump (m)",
    "Pole Vault (m)",
    "Shot Put (m)",
    "Discus (m)",
    "Javelin (m)",
    "XC (s)",
    "Performance count",
    "Meet count",
    "Head TF Coach",
    "Head TF Coach Email",
    "Head XC Coach",
    "Head XC Coach Email",
    "Athletic Director",
    "AD Email",
    "All Emails (school)",
    "Preferred Recruiting Contact",
    "Preferred Contact Role",
    "Preferred Contact Email",
    "Contact Coverage State",
    "Athletic.net URL",
    "MileSplit URL",
    "Other profile URLs",
    "Sources Count",
    "Identity Confidence",
    "Coverage State",
    "Conflict Flag",
    "Review Flag",
];

/// Column widths, one per header.
pub(super) const WIDTHS: [u16; 51] = [
    20, 26, 10, 16, 14, 8, 30, 20, 8, 8, 8, 12, 12, 12, 12, 12, 12, 12, 12, 18, 18, 18, 16, 16, 16,
    16, 16, 16, 16, 12, 16, 12, 24, 32, 24, 32, 24, 32, 48, 30, 24, 32, 28, 36, 36, 40, 14, 18, 18,
    14, 14,
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
    cells.extend(school_cells(contacts, director));
    cells.extend(contact_cells(contacts, preferred));
    cells.extend(profile_cells(profiles));
    cells.extend(audit_cells(dataset, athlete, tally, &prs)?);
    Ok(cells)
}

/// The identity columns: stored id, name, gender, cohort year and grade, then school placement.
fn identity_cells(dataset: &Dataset, athlete: &CanonicalAthlete) -> Vec<Cell> {
    let school = athlete.school.as_str();
    row!(
        Cell::text(athlete.id.as_str()),
        Cell::text(athlete.canonical_name.clone()),
        Cell::text(athlete.gender.stable_key().to_string()),
        Cell::Number(f64::from(athlete.grad_year.get())),
        current_grade(athlete),
        Cell::text(dataset.school_state(school)),
        Cell::text(dataset.school_name(school)),
        Cell::text(dataset.school_city(school)),
    )
}

const PR_EVENTS: [&str; 19] = [
    "Track100m",
    "Track200m",
    "Track400m",
    "Track800m",
    "Track1600m",
    "Track3200m",
    "Track1Mile",
    "Track5000m",
    "Track100mHurdles",
    "Track110mHurdles",
    "Track300mHurdles",
    "HighJump",
    "LongJump",
    "TripleJump",
    "PoleVault",
    "ShotPut",
    "Discus",
    "Javelin",
    "CrossCountry",
];

/// Sport flags, the supported personal-best columns, and the stored performance and meet counts.
fn participation_cells(
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&PrRow],
) -> ReportResult<Vec<Cell>> {
    let mut cells = row!(
        flag(athlete.sports.contains(&Sport::CrossCountry)),
        flag(athlete.sports.contains(&Sport::IndoorTrack)),
        flag(athlete.sports.contains(&Sport::OutdoorTrack)),
    );
    cells.extend(PR_EVENTS.iter().map(|event| {
        prs.iter()
            .find(|pr| pr.event == *event)
            .and_then(|pr| mark_value(&pr.source_mark))
            .map_or(Cell::Empty, Cell::Number)
    }));
    cells.push(Cell::number(tally.map_or(0, |tally| tally.performances))?);
    cells.push(Cell::number(tally.map_or(0, |tally| tally.meets.len()))?);
    Ok(cells)
}

/// The three profile-URL columns, in the order [`Profiles`] splits them.
fn profile_cells(profiles: Profiles) -> Vec<Cell> {
    row!(
        published(profiles.athletic_net),
        published(profiles.milesplit),
        Cell::text(profiles.other.join("; ")),
    )
}

/// The school-level contact names and their field-specific addresses.
fn school_cells(contacts: Option<&SchoolContacts>, director: Option<&Named>) -> Vec<Cell> {
    row!(
        published(contacts.and_then(|contacts| contacts.head_track.clone())),
        published(contacts.and_then(|contacts| contacts.head_track_email.clone())),
        published(contacts.and_then(|contacts| contacts.head_cross_country.clone())),
        published(contacts.and_then(|contacts| contacts.head_cross_country_email.clone())),
        published(director.map(|director| director.name.clone())),
        published(director.and_then(|director| director.email.clone())),
    )
}

/// The five audit columns: source count, confidence, coverage, conflict, and review.
fn audit_cells(
    _dataset: &Dataset,
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&PrRow],
) -> ReportResult<Vec<Cell>> {
    Ok(row!(
        Cell::number(source_count(athlete))?,
        Cell::Number(f64::from(athlete.identity_confidence.get())),
        Cell::text(coverage_state(
            tally.is_some_and(|tally| tally.performances > 0),
            !prs.is_empty()
        )),
        flag(conflicts(athlete)),
        flag(athlete.identity_confidence < Confidence::HIGH),
    ))
}

/// The school-wide email inventory, then the preferred recruiting contact ladder.
fn contact_cells(contacts: Option<&SchoolContacts>, preferred: Preferred) -> Vec<Cell> {
    row!(
        Cell::text(
            contacts
                .map(|contacts| contacts.all_emails.clone())
                .unwrap_or_default(),
        ),
        Cell::text(preferred.name),
        Cell::text(preferred.role),
        Cell::text(preferred.email),
        Cell::text(preferred.state.as_str()),
    )
}
