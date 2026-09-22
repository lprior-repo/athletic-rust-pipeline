//! The `Coaches` sheet (objective §53): one row per canonical coach in the store.
//!
//! Published columns, in this order: School ID, School, State, Sport, Coach, Role, Professional Email,
//! Athletic Director, AD Professional Email, Official Source URL, Observed Date.
//!
//! Every cell is a stored field:
//!
//! * `Sport` is the coach's own `Sport` binding spelled the way the domain spells it, and
//!   `school_wide` for the school-wide roles (`CanonicalCoach::sport` is `None` for an athletic
//!   director) — the same label the census's own coach-sport counter uses;
//! * `Role` is the coach's `CoachRole`;
//! * `Professional Email` is the published address the collection contract kept: a consumer mailbox
//!   is dropped at merge time, so a blank here is a withheld or absent mailbox, never a personal one;
//! * `Athletic Director` and `AD Professional Email` name the school's own AD row, so a coach row and
//!   an AD row for one school cite each other;
//! * `Official Source URL` is the first evidence source URL stored for the coach, else the first
//!   source-identity URL; `Observed Date` is the newest date in the coach's evidence.

use crate::report::ReportResult;
use census_domain::model::CanonicalCoach;

use super::super::cells::{cell, row, Cell};
use super::dataset::Dataset;

/// The worksheet name, as objective §53 publishes it.
pub(super) const TITLE: &str = "Coaches";

/// The sheet's column headers, in published order.
pub(super) const HEADERS: [&str; 11] = [
    "School ID",
    "School",
    "State",
    "Sport",
    "Coach",
    "Role",
    "Professional Email",
    "Athletic Director",
    "AD Professional Email",
    "Official Source URL",
    "Observed Date",
];

/// Column widths, one per header.
pub(super) const WIDTHS: [u16; 11] = [20, 30, 8, 14, 26, 16, 32, 26, 32, 40, 14];

/// The `Coaches` sheet, ordered by state, school, sport, role, then coach.
pub(super) fn sheet(dataset: &Dataset) -> ReportResult<Vec<Vec<Cell>>> {
    let mut ordered: Vec<(&CanonicalCoach, SortKey)> = dataset
        .coaches
        .iter()
        .map(|coach| (coach, sort_key(dataset, coach)))
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for (coach, _) in ordered {
        rows.push(row_for(dataset, coach));
    }
    Ok(rows)
}

/// The five fields the sheet orders by.
type SortKey = (String, String, String, String, String);

fn sort_key(dataset: &Dataset, coach: &CanonicalCoach) -> SortKey {
    (
        dataset.school_state(coach.school.as_str()),
        dataset.school_name(coach.school.as_str()),
        sport_label(coach),
        role_label(coach),
        coach.name.clone(),
    )
}

/// One coach's published row.
fn row_for(dataset: &Dataset, coach: &CanonicalCoach) -> Vec<Cell> {
    let contacts = dataset.contacts.get(coach.school.as_str());
    row!(
        Cell::text(coach.school.as_str()),
        Cell::text(dataset.school_name(coach.school.as_str())),
        Cell::text(dataset.school_state(coach.school.as_str())),
        Cell::text(sport_label(coach)),
        Cell::text(coach.name.clone()),
        Cell::text(role_label(coach)),
        Cell::text(coach.professional_email.clone().unwrap_or_default()),
        Cell::text(
            contacts
                .and_then(|contacts| contacts.director.clone())
                .unwrap_or_default()
        ),
        Cell::text(
            contacts
                .and_then(|contacts| contacts.director_email.clone())
                .unwrap_or_default()
        ),
        Cell::text(official_url(coach)),
        Cell::text(observed_date(coach)),
    )
}

/// The coach's sport binding, or `school_wide` for a role that is not bound to one sport.
fn sport_label(coach: &CanonicalCoach) -> String {
    coach
        .sport
        .map(|sport| format!("{sport:?}"))
        .unwrap_or_else(|| "school_wide".to_string())
}

fn role_label(coach: &CanonicalCoach) -> String {
    format!("{:?}", coach.role)
}

/// The document the contact was read from: a stored evidence source URL, else the source identity's
/// own URL.
fn official_url(coach: &CanonicalCoach) -> String {
    coach
        .evidence
        .iter()
        .find_map(|evidence| evidence.source.url.clone())
        .or_else(|| {
            coach
                .source_identities
                .iter()
                .find_map(|identity| identity.url.clone())
        })
        .unwrap_or_default()
}

/// The newest ISO-8601 observation date in the coach's evidence; blank when no evidence carries one.
fn observed_date(coach: &CanonicalCoach) -> String {
    coach
        .evidence
        .iter()
        .map(|evidence| evidence.observed_on.as_str())
        .max()
        .unwrap_or_default()
        .to_string()
}
