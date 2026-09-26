//! The `Athletes` sheet (objective §50): one row per canonical athlete in the run's cohort.
//!
//! The column rules and metadata live in the `rules` submodule; the `cells` submodule
//! provides the pure cell helpers. The remaining items here depend on the workbook `Dataset`.

mod cells;
mod rules;

pub(crate) use rules::{HEADERS, TITLE, WIDTHS};

use super::super::cells::{row, Cell};
use super::columns::{
    coverage_state, flag, observed_grade, observed_school_year, published, source_count,
};
use super::identity::has_conflicts;
use super::contact::{self, Preferred, SchoolContacts};
use super::dataset::Dataset;
use super::facts::AthleteTally;
use super::profiles::profiles_of;
use super::prs::PrRow;
use crate::report::ReportResult;
use cells::{
    event_list, gpa_source, headline_pr_summary, participation_flags, participation_metrics,
    pr_event_cells, profile_cells, public_recruiting_gpa, tf_flag,
};
use census_domain::model::{CanonicalAthlete, Confidence};

/// A human-readable confidence label: `"high"`, `"medium"`, `"low"`, or `"unknown"` when the
/// confidence has not been set (the zero-value `Confidence` that carries no measured value).
fn confidence_label(athlete: &CanonicalAthlete) -> &'static str {
    match athlete.identity_confidence {
        Confidence::HIGH => "high",
        Confidence::MEDIUM => "medium",
        Confidence::LOW => "low",
        c if c.get() == 0 => "unknown",
        _ => "unknown",
    }
}

/// The review status column: `"verified"` when identity_confidence is HIGH, there are no
/// conflicts (grade, identity, or retained), and no unresolved review cases; `"review"` when
/// the athlete carries low confidence, has unresolved conflicts, or has a pending/rejected
/// review case; `"unknown"` when the decision state cannot be determined from available records.
fn review_status(athlete: &CanonicalAthlete) -> Cell {
    let decision = super::identity::IdentityDecision::from_athlete(athlete);
    let has_conflicts = has_conflicts(athlete);
    match decision {
        super::identity::IdentityDecision::Accepted => Cell::text("verified"),
        super::identity::IdentityDecision::Rejected | super::identity::IdentityDecision::Unresolved => Cell::text("review"),
        super::identity::IdentityDecision::NoDecision if !has_conflicts && athlete.identity_confidence >= Confidence::HIGH => Cell::text("verified"),
        _ => Cell::text("review"),
    }
}

/// The `Athletes` sheet, ordered by state, school, then athlete.
pub(super) fn sheet(dataset: &Dataset) -> ReportResult<Vec<Vec<Cell>>> {
    let mut ordered: Vec<(&CanonicalAthlete, (String, String, String))> = dataset
        .athletes
        .iter()
        .map(|athlete| {
            let school = athlete.school.as_str();
            (
                athlete,
                (
                    dataset.school_state(school),
                    dataset.school_name(school),
                    athlete.canonical_name.clone(),
                ),
            )
        })
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for (athlete, _) in ordered {
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
    cells.extend(tf_flag(athlete));
    cells.extend(participation_flags(athlete));
    cells.extend(event_list(athlete, &prs));
    cells.extend(headline_pr_summary(athlete, &prs));
    cells.extend(pr_event_cells(&prs));
    cells.extend(participation_metrics(tally)?);
    cells.push(published(contacts.and_then(|c| c.head_track.clone())));
    cells.push(published(contacts.and_then(|c| c.head_track_email.clone())));
    cells.push(published(
        contacts.and_then(|c| c.head_cross_country.clone()),
    ));
    cells.push(published(
        contacts.and_then(|c| c.head_cross_country_email.clone()),
    ));
    cells.extend(coach_professional_email(contacts, &preferred));
    cells.push(published(director.map(|d| d.name.clone())));
    cells.push(published(director.and_then(|d| d.email.clone())));
    cells.extend(school_athletics_url(dataset, school));
    cells.extend(public_recruiting_gpa());
    cells.extend(gpa_source());
    cells.extend(contact_cells(contacts, preferred));
    cells.extend(profile_cells(profiles));
    cells.extend(audit_cells(dataset, athlete, tally, &prs)?);
    Ok(cells)
}

/// The identity columns: stored id, name, gender, cohort year and grade, then school placement.
fn identity_cells(dataset: &Dataset, athlete: &CanonicalAthlete) -> Vec<Cell> {
    let school = athlete.school.as_str();
    let school_id = dataset
        .schools
        .get(school)
        .map(|s| s.id.as_str())
        .unwrap_or(school)
        .to_string();
    row!(
        Cell::text(athlete.id.as_str()),
        Cell::text(athlete.canonical_name.clone()),
        Cell::text(athlete.gender.stable_key().to_string()),
        Cell::Number(f64::from(athlete.grad_year.get())),
        observed_grade(athlete),
        observed_school_year(athlete),
        Cell::text(dataset.school_state(school)),
        Cell::text(dataset.school_name(school)),
        Cell::text(school_id),
        Cell::text(dataset.school_city(school)),
    )
}

/// `Coach Professional Email` — the head TF coach's professional email; when absent, the head
/// XC coach's; when that is absent, the preferred recruiting contact's email if that contact's
/// role is a coaching role; blank otherwise.
fn coach_professional_email(contacts: Option<&SchoolContacts>, preferred: &Preferred) -> Vec<Cell> {
    let tf_email = contacts.and_then(|c| c.head_track_email.clone());
    let xc_email = contacts.and_then(|c| c.head_cross_country_email.clone());
    let email = tf_email.or(xc_email).or_else(|| {
        if preferred.role.starts_with("Head") {
            Some(preferred.email.clone())
        } else {
            None
        }
    });
    vec![published(email)]
}

/// `School Athletics URL` — the stored `athletics_website` for the athlete's school;
/// blank when the store holds none.
fn school_athletics_url(dataset: &Dataset, school: &str) -> Vec<Cell> {
    let url = dataset
        .schools
        .get(school)
        .and_then(|s| s.athletics_website.clone());
    vec![published(url)]
}

/// The five audit columns: sources count, confidence label, coverage, conflict flag, and review status.
fn audit_cells(
    _dataset: &Dataset,
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&PrRow],
) -> ReportResult<Vec<Cell>> {
    Ok(row!(
        Cell::number(source_count(athlete))?,
        Cell::text(confidence_label(athlete)),
        Cell::text(coverage_state(
            tally.is_some_and(|t| t.performances > 0),
            !prs.is_empty()
        )),
        flag(has_conflicts(athlete)),
        review_status(athlete),
    ))
}

/// The school-wide email inventory, then the preferred recruiting contact ladder.
fn contact_cells(contacts: Option<&SchoolContacts>, preferred: Preferred) -> Vec<Cell> {
    row!(
        Cell::text(contacts.map(|c| c.all_emails.clone()).unwrap_or_default(),),
        Cell::text(preferred.name),
        Cell::text(preferred.role),
        Cell::text(preferred.email),
        Cell::text(preferred.state.as_str()),
    )
}
