use census_domain::model::{
    CanonicalAthlete, GradYear, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
    COHORT_EVIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY, CONTACT_CONFLICT_FAMILY,
    IDENTITY_UNVERIFIED_FAMILY, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY,
};
use census_review::ReviewFamily;
use std::collections::HashMap;

use crate::report::{ReportResult, Scope};
use crate::workbook::cells::{row, Cell};
use census_store::Store;

use super::{school_name_index, subject_of, Family, QueueRow, StoreRows};

mod conflicts;
mod review;

use conflicts::{athlete_identity, cohort_evidence, contact_conflicts, school_identity};
use review::{cohort_unverified, identity_unverified, unresolved_schools, unresolved_venues};

pub(super) const CONFLICT_WIDTHS: [u16; 5] = [34, 12, 34, 44, 96];

pub(super) const REVIEW_WIDTHS: [u16; 7] = [34, 12, 34, 44, 24, 12, 96];

pub(super) fn conflicts_sheet(conflicts: &[Family], rows: &StoreRows) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!("Family", "State", "Subject ID", "Subject", "Detail")];
    for family in conflicts {
        for retained in &family.rows {
            let state = state_for_subject_id(&retained.subject_id, rows);
            cells.push(row!(
                Cell::text(family.label),
                state,
                Cell::text(&retained.subject_id),
                Cell::text(&retained.subject),
                Cell::text(&retained.detail)
            ));
        }
    }
    cells
}

pub(super) fn review_sheet(review: &[Family], rows: &StoreRows) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!(
        "Family",
        "State",
        "Subject ID",
        "Subject",
        "Answer",
        "Confidence",
        "Detail"
    )];
    for family in review {
        for retained in &family.rows {
            let state = state_for_subject(family.label, &retained.subject_id, rows);
            cells.push(row!(
                Cell::text(family.label),
                state,
                Cell::text(&retained.subject_id),
                Cell::text(&retained.subject),
                Cell::Empty,
                Cell::Empty,
                Cell::text(&retained.detail)
            ));
        }
    }
    cells.extend(
        rows.verdicts
            .iter()
            .map(|verdict| verdict_review_row(verdict, rows)),
    );
    cells
}

fn state_for_subject_id(subject_id: &str, rows: &StoreRows) -> Cell {
    rows.schools
        .iter()
        .find(|school| school.id.as_str() == subject_id)
        .and_then(|school| school.state)
        .or_else(|| {
            rows.athletes
                .iter()
                .find(|athlete| athlete.id.as_str() == subject_id)
                .and_then(|athlete| {
                    rows.schools
                        .iter()
                        .find(|school| school.id == athlete.school)
                        .and_then(|school| school.state)
                })
        })
        .map_or(Cell::Empty, |state| Cell::text(state.code()))
}

fn verdict_review_row(verdict: &ReviewVerdictRecord, rows: &StoreRows) -> Vec<Cell> {
    let family = ReviewFamily::parse(&verdict.family).map_or_else(
        || verdict.family.clone(),
        |family| family.label().to_string(),
    );
    let subject = verdict_subject(verdict, rows);
    let answer = match (verdict.field.is_empty(), verdict.value.is_empty()) {
        (false, false) => format!("{}={}", verdict.field, verdict.value),
        _ => String::new(),
    };
    row!(
        Cell::text(family),
        state_for_subject(&verdict.family, &verdict.subject_id, rows),
        Cell::text(&verdict.subject_id),
        Cell::text(subject),
        Cell::text(answer),
        Cell::Number(f64::from(verdict.confidence)),
        Cell::text(&verdict.rationale)
    )
}

fn verdict_subject(verdict: &ReviewVerdictRecord, rows: &StoreRows) -> String {
    match ReviewFamily::parse(&verdict.family) {
        Some(ReviewFamily::SchoolJurisdiction) => rows
            .schools
            .iter()
            .find(|school| school.id.as_str() == verdict.subject_id)
            .map_or_else(|| verdict.subject_id.clone(), |school| school.name.clone()),
        Some(ReviewFamily::MeetJurisdiction) => rows
            .meets
            .iter()
            .find(|meet| meet.id.as_str() == verdict.subject_id)
            .map_or_else(|| verdict.subject_id.clone(), |meet| meet.name.clone()),
        Some(ReviewFamily::AthleteIdentity) => rows
            .athletes
            .iter()
            .find(|athlete| athlete.id.as_str() == verdict.subject_id)
            .map_or_else(
                || verdict.subject_id.clone(),
                |athlete| {
                    subject_of(
                        &athlete.canonical_name,
                        rows.schools
                            .iter()
                            .find(|school| school.id == athlete.school)
                            .map(|school| school.name.as_str()),
                    )
                },
            ),
        None => verdict.subject_id.clone(),
    }
}

fn state_for_subject(family: &str, subject_id: &str, rows: &StoreRows) -> Cell {
    match ReviewFamily::parse(family) {
        Some(ReviewFamily::SchoolJurisdiction) => rows
            .schools
            .iter()
            .find(|school| school.id.as_str() == subject_id)
            .and_then(|school| school.state)
            .map_or(Cell::Empty, |state| Cell::text(state.code())),
        Some(ReviewFamily::MeetJurisdiction) => rows
            .meets
            .iter()
            .find(|meet| meet.id.as_str() == subject_id)
            .and_then(|meet| meet.state)
            .map_or(Cell::Empty, |state| Cell::text(state.code())),
        Some(ReviewFamily::AthleteIdentity) => rows
            .athletes
            .iter()
            .find(|athlete| athlete.id.as_str() == subject_id)
            .and_then(|athlete| {
                rows.schools
                    .iter()
                    .find(|school| school.id == athlete.school)
                    .and_then(|school| school.state)
            })
            .map_or(Cell::Empty, |state| Cell::text(state.code())),
        None => Cell::Empty,
    }
}

pub(super) const COHORT_EVIDENCE: &str = COHORT_EVIDENCE_FAMILY;
pub(super) const ATHLETE_IDENTITY: &str = ATHLETE_IDENTITY_FAMILY;
pub(super) const SCHOOL_IDENTITY: &str = SCHOOL_IDENTITY_FAMILY;
pub(super) const CONTACT_CONFLICT: &str = CONTACT_CONFLICT_FAMILY;
pub(super) const COHORT_UNVERIFIED: &str = COHORT_UNVERIFIED_FAMILY;
pub(super) const IDENTITY_UNVERIFIED: &str = IDENTITY_UNVERIFIED_FAMILY;
pub(super) const UNRESOLVED_VENUE: &str = UNRESOLVED_VENUE_FAMILY;
pub(super) const UNRESOLVED_SCHOOL: &str = UNRESOLVED_SCHOOL_FAMILY;

pub(super) fn conflict_families(rows: &StoreRows, names: &HashMap<&str, &str>) -> Vec<Family> {
    vec![
        cohort_evidence(rows, names),
        athlete_identity(rows, names),
        school_identity(&rows.schools),
        contact_conflicts(rows, names),
    ]
}

pub(super) fn review_families(
    rows: &StoreRows,
    names: &HashMap<&str, &str>,
) -> ReportResult<Vec<Family>> {
    Ok(vec![
        cohort_unverified(rows, names),
        identity_unverified(rows, names)?,
        unresolved_venues(&rows.meets),
        unresolved_schools(&rows.schools),
    ])
}

fn class_of_2027(athletes: &[CanonicalAthlete]) -> impl Iterator<Item = &CanonicalAthlete> + '_ {
    athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
}

fn queue_row(id: &str, subject: String, detail: String) -> QueueRow {
    QueueRow {
        subject_id: id.to_string(),
        subject,
        detail,
    }
}

pub fn retained_records(store: &Store) -> ReportResult<RetainedRecords> {
    use census_store::clock::Clock;
    let today = census_store::clock::SystemClock.today();
    let school_year = census_domain::model::SchoolYear::from_date(&today).ok_or_else(|| {
        crate::report::ReportError::Invariant {
            detail: format!("cannot determine contact school year from {today}"),
        }
    })?;
    let rows = StoreRows::read(store, Scope::AllSources, school_year)?;
    let names = school_name_index(&rows.schools);
    Ok(RetainedRecords {
        conflicts: labelled(conflict_families(&rows, &names)),
        reviews: labelled(review_families(&rows, &names)?),
    })
}

#[derive(Debug, Default)]
pub struct RetainedRecords {
    pub conflicts: Vec<(&'static str, QueueRow)>,
    pub reviews: Vec<(&'static str, QueueRow)>,
}

fn labelled(families: Vec<Family>) -> Vec<(&'static str, QueueRow)> {
    let mut out = Vec::new();
    for family in families {
        for row in family.rows {
            out.push((family.label, row));
        }
    }
    out
}
