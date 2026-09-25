//! The retained queues: conflicts between rows the merge kept separate, and the material a human or
//! the review model still has to adjudicate.
//!
//! Both queues render retained rows rather than dropping them or collapsing them into a count: a
//! school another school's normalized name collides with, an athlete whose own grade observations
//! disagree about the graduating class, or a meet whose venue was never placed.
//! Each family prints one row per retained subject — the same subject ids the store holds — so the
//! operator acts on rows instead of on a number. The families themselves live in `conflicts` and
//! `review`; this module holds the labels, the family lists and the one row shape they share.
//!
//! The families are deliberately narrow: a row appears here because a *stored* field is unresolved,
//! never because a heuristic disliked it. Cohort-family rows are scoped to the published class of
//! 2027 (the cohort the census document counts); school and meet rows cover the whole table, because
//! neither carries a cohort.

use census_domain::model::{
    CanonicalAthlete, GradYear, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
    COHORT_EVIDENCE_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY,
    CONTACT_CONFLICT_FAMILY, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
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
use review::{cohort_unverified, low_confidence, unresolved_schools, unresolved_venues};

/// Widths for the consolidated data-quality sheet.
pub(super) const DATA_QUALITY_WIDTHS: [u16; 8] = [12, 34, 12, 34, 34, 24, 12, 96];

/// Render retained queues followed by durable model verdicts in one operator sheet.
pub(super) fn data_quality_sheet(
    conflicts: &[Family],
    review: &[Family],
    rows: &StoreRows,
) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!(
        "Queue",
        "Family",
        "State",
        "Subject ID",
        "Subject",
        "Answer",
        "Confidence",
        "Detail"
    )];
    for family in conflicts {
        for retained in &family.rows {
            cells.push(queue_quality_row(
                "Conflict",
                family.label,
                retained,
                Cell::Empty,
            ));
        }
    }
    for family in review {
        for retained in &family.rows {
            let state = state_for_subject(family.label, &retained.subject_id, rows);
            cells.push(queue_quality_row("Review", family.label, retained, state));
        }
    }
    cells.extend(
        rows.verdicts
            .iter()
            .map(|verdict| verdict_quality_row(verdict, rows)),
    );
    cells
}

fn queue_quality_row(queue: &str, family: &str, retained: &QueueRow, state: Cell) -> Vec<Cell> {
    row!(
        Cell::text(queue),
        Cell::text(family),
        state,
        Cell::text(&retained.subject_id),
        Cell::text(&retained.subject),
        Cell::Empty,
        Cell::Empty,
        Cell::text(&retained.detail)
    )
}

fn verdict_quality_row(verdict: &ReviewVerdictRecord, rows: &StoreRows) -> Vec<Cell> {
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
        "Verdict",
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

/// Family labels, shared with the reconciliation block on `Run Metrics`.
///
/// The names themselves live with the record they label (`census_domain::model`), so the lane that
/// matches a retained case by name, the workbook that prints it and the seal that counts it cannot
/// drift apart.
pub(super) const COHORT_EVIDENCE: &str = COHORT_EVIDENCE_FAMILY;
pub(super) const ATHLETE_IDENTITY: &str = ATHLETE_IDENTITY_FAMILY;
pub(super) const SCHOOL_IDENTITY: &str = SCHOOL_IDENTITY_FAMILY;
pub(super) const CONTACT_CONFLICT: &str = CONTACT_CONFLICT_FAMILY;
pub(super) const COHORT_UNVERIFIED: &str = COHORT_UNVERIFIED_FAMILY;
pub(super) const LOW_CONFIDENCE: &str = COHORT_IDENTITY_CONFIDENCE_FAMILY;
pub(super) const UNRESOLVED_VENUE: &str = UNRESOLVED_VENUE_FAMILY;
pub(super) const UNRESOLVED_SCHOOL: &str = UNRESOLVED_SCHOOL_FAMILY;

/// Every conflict family the store retains.
pub(super) fn conflict_families(rows: &StoreRows, names: &HashMap<&str, &str>) -> Vec<Family> {
    vec![
        cohort_evidence(rows, names),
        athlete_identity(rows, names),
        school_identity(&rows.schools),
        contact_conflicts(rows, names),
    ]
}

/// Every review family the store retains.
pub(super) fn review_families(rows: &StoreRows, names: &HashMap<&str, &str>) -> Vec<Family> {
    vec![
        cohort_unverified(rows, names),
        low_confidence(rows, names),
        unresolved_venues(&rows.meets),
        unresolved_schools(&rows.schools),
    ]
}

/// The published cohort's athlete rows: the same class the census document counts.
fn class_of_2027(athletes: &[CanonicalAthlete]) -> impl Iterator<Item = &CanonicalAthlete> + '_ {
    athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
}

/// One retained row: the subject's id and name, and why the row is unresolved.
fn queue_row(id: &str, subject: String, detail: String) -> QueueRow {
    QueueRow {
        subject_id: id.to_string(),
        subject,
        detail,
    }
}

/// The retained rows of both queues as `(family label, row)` pairs: the durable record the store's
/// `conflicts` and `review_cases` tables hold, read through the same families the sheets render, so
/// the store and the workbook can never name different findings.
pub fn retained_records(store: &Store) -> ReportResult<RetainedRecords> {
    let rows = StoreRows::read(store, Scope::AllSources)?;
    let names = school_name_index(&rows.schools);
    Ok(RetainedRecords {
        conflicts: labelled(conflict_families(&rows, &names)),
        reviews: labelled(review_families(&rows, &names)),
    })
}

/// The retained conflicts and reviews, each row paired with the family that produced it.
#[derive(Debug, Default)]
pub struct RetainedRecords {
    pub conflicts: Vec<(&'static str, QueueRow)>,
    pub reviews: Vec<(&'static str, QueueRow)>,
}

/// Flatten families into `(label, row)` pairs, keeping the family order the sheets use.
fn labelled(families: Vec<Family>) -> Vec<(&'static str, QueueRow)> {
    let mut out = Vec::new();
    for family in families {
        for row in family.rows {
            out.push((family.label, row));
        }
    }
    out
}
