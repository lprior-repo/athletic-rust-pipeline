use census_domain::model::{
    CanonicalAthlete, GradYear, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY,
    COHORT_EVIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY, CONTACT_CONFLICT_FAMILY,
    IDENTITY_UNVERIFIED_FAMILY, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY, UNSUPPORTED_GRADUATION_FAMILY,
};
use census_review::ReviewFamily;
use std::collections::HashMap;

use crate::report::{ReportResult, Scope};
use crate::workbook::cells::{row, Cell};

use super::{school_name_index, Family, QueueRow, StoreRows, SubjectIndex};

mod conflicts;
mod review;

use conflicts::{athlete_identity, cohort_evidence, contact_conflicts, school_identity};
use review::{cohort_unverified, identity_unverified, unresolved_schools, unresolved_venues};

pub(super) const CONFLICT_WIDTHS: [u16; 5] = [34, 12, 34, 44, 96];

pub(super) const REVIEW_WIDTHS: [u16; 7] = [34, 12, 34, 44, 24, 12, 96];

pub(super) fn conflicts_sheet(conflicts: &[Family], index: &SubjectIndex<'_>) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!("Family", "State", "Subject ID", "Subject", "Detail")];
    for family in conflicts {
        for retained in &family.rows {
            let state = state_for_subject_id(&retained.subject_id, index);
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

pub(super) fn review_sheet(
    review: &[Family],
    rows: &StoreRows,
    index: &SubjectIndex<'_>,
) -> Vec<Vec<Cell>> {
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
            let state = state_for_subject(family.label, &retained.subject_id, index);
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
            .map(|verdict| verdict_review_row(verdict, index)),
    );
    cells
}

fn state_for_subject_id(subject_id: &str, index: &SubjectIndex<'_>) -> Cell {
    index
        .state(subject_id)
        .map_or(Cell::Empty, |state| Cell::text(state.code()))
}

fn verdict_review_row(verdict: &ReviewVerdictRecord, index: &SubjectIndex<'_>) -> Vec<Cell> {
    let family = ReviewFamily::parse(&verdict.family).map_or_else(
        || verdict.family.clone(),
        |family| family.label().to_string(),
    );
    let subject = verdict_subject(verdict, index);
    let answer = match (verdict.field.is_empty(), verdict.value.is_empty()) {
        (false, false) => format!("{}={}", verdict.field, verdict.value),
        _ => String::new(),
    };
    row!(
        Cell::text(family),
        state_for_subject(&verdict.family, &verdict.subject_id, index),
        Cell::text(&verdict.subject_id),
        Cell::text(subject),
        Cell::text(answer),
        Cell::Number(f64::from(verdict.confidence)),
        Cell::text(&verdict.rationale)
    )
}

fn verdict_subject(verdict: &ReviewVerdictRecord, index: &SubjectIndex<'_>) -> String {
    match ReviewFamily::parse(&verdict.family) {
        Some(ReviewFamily::SchoolJurisdiction | ReviewFamily::SchoolLink) => index
            .school(&verdict.subject_id)
            .map_or_else(|| verdict.subject_id.clone(), |school| school.name.clone()),
        Some(ReviewFamily::MeetJurisdiction) => index
            .meet(&verdict.subject_id)
            .map_or_else(|| verdict.subject_id.clone(), |meet| meet.name.clone()),
        Some(ReviewFamily::AthleteIdentity) => index.subject(&verdict.subject_id),
        None => verdict.subject_id.clone(),
    }
}

fn state_for_subject(family: &str, subject_id: &str, index: &SubjectIndex<'_>) -> Cell {
    let state = match ReviewFamily::parse(family) {
        Some(ReviewFamily::SchoolJurisdiction | ReviewFamily::SchoolLink) => {
            index.school_state(subject_id)
        }
        Some(ReviewFamily::MeetJurisdiction) => index.meet_state(subject_id),
        Some(ReviewFamily::AthleteIdentity) => index.athlete_state(subject_id),
        None => None,
    };
    state.map_or(Cell::Empty, |state| Cell::text(state.code()))
}

pub(super) const COHORT_EVIDENCE: &str = COHORT_EVIDENCE_FAMILY;
pub(super) const ATHLETE_IDENTITY: &str = ATHLETE_IDENTITY_FAMILY;
pub(super) const SCHOOL_IDENTITY: &str = SCHOOL_IDENTITY_FAMILY;
pub(super) const CONTACT_CONFLICT: &str = CONTACT_CONFLICT_FAMILY;
pub(super) const COHORT_UNVERIFIED: &str = COHORT_UNVERIFIED_FAMILY;
pub(super) const IDENTITY_UNVERIFIED: &str = IDENTITY_UNVERIFIED_FAMILY;
pub(super) const UNRESOLVED_VENUE: &str = UNRESOLVED_VENUE_FAMILY;
pub(super) const UNRESOLVED_SCHOOL: &str = UNRESOLVED_SCHOOL_FAMILY;

pub(in crate::workbook) fn conflict_families(
    rows: &StoreRows,
    cohort: &[CanonicalAthlete],
    names: &HashMap<&str, &str>,
) -> Vec<Family> {
    vec![
        cohort_evidence(cohort, names),
        athlete_identity(cohort, names),
        school_identity(rows.schools),
        contact_conflicts(rows, names),
    ]
}

pub(in crate::workbook) fn review_families(
    rows: &StoreRows,
    cohort: &[CanonicalAthlete],
    names: &HashMap<&str, &str>,
) -> ReportResult<Vec<Family>> {
    Ok(vec![
        cohort_unverified(cohort, names),
        identity_unverified(rows, cohort, names)?,
        unresolved_venues(rows.meets),
        unresolved_schools(rows.schools),
        school_links(rows),
        unsupported_graduation(rows),
    ])
}

fn school_links(rows: &StoreRows) -> Family {
    let mut family = Family::new(SCHOOL_IDENTITY);
    for case in rows
        .review_cases
        .iter()
        .filter(|case| case.family == SCHOOL_IDENTITY_FAMILY && case.state == ReviewState::Pending)
    {
        family.push(queue_row(
            &case.subject_id,
            case.subject.clone(),
            case.detail.clone(),
        ));
    }
    family
}

fn unsupported_graduation(rows: &StoreRows) -> Family {
    let mut family = Family::new(UNSUPPORTED_GRADUATION_FAMILY);
    for case in rows.review_cases.iter().filter(|case| {
        case.family == UNSUPPORTED_GRADUATION_FAMILY && case.state == ReviewState::Pending
    }) {
        family.push(queue_row(
            &case.subject_id,
            case.subject.clone(),
            case.detail.clone(),
        ));
    }
    family
}

pub(in crate::workbook) fn cohort_of(
    dataset: &crate::export::ExportDataset,
    scope: Scope,
) -> Vec<CanonicalAthlete> {
    crate::report::cohort_candidates(dataset, scope, Some(GradYear::CO2027.get()))
}

fn queue_row(id: &str, subject: String, detail: String) -> QueueRow {
    QueueRow {
        subject_id: id.to_string(),
        subject,
        detail,
    }
}

pub fn retained_records(dataset: &crate::export::ExportDataset) -> ReportResult<RetainedRecords> {
    let generated_on = &dataset.lineage.generated_on;
    let school_year =
        census_domain::model::SchoolYear::from_date(generated_on).ok_or_else(|| {
            crate::report::ReportError::Invariant {
                detail: format!("cannot determine contact school year from {generated_on}"),
            }
        })?;
    let derivation = crate::report::Derivation::of(dataset, Scope::AllSources, None);
    let rows = StoreRows::of(&derivation, school_year)?;
    let cohort = cohort_of(dataset, Scope::AllSources);
    let names = school_name_index(rows.schools);
    Ok(RetainedRecords {
        conflicts: labelled(conflict_families(&rows, &cohort, &names)),
        reviews: labelled(review_families(&rows, &cohort, &names)?),
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
