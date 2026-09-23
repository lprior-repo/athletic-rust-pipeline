//! The retained queues: conflicts between rows the merge kept separate, and the material a human or
//! the review model still has to adjudicate.
//!
//! Both sheets render the retained rows rather than dropping them or collapsing them into a count: a
//! school another school's normalized name collides with, an athlete whose own grade observations
//! disagree about the graduating class, a meet whose venue was never placed, a coach whose only
//! published address was a personal mailbox and was therefore withheld by the collection contract.
//! Each family prints one row per retained subject — the same subject ids the store holds — so the
//! operator acts on rows instead of on a number. The families themselves live in [`conflicts`] and
//! [`review`]; this module holds the labels, the family lists and the one row shape they share.
//!
//! The families are deliberately narrow: a row appears here because a *stored* field is unresolved,
//! never because a heuristic disliked it. Cohort-family rows are scoped to the published class of
//! 2027 (the cohort the census document counts); school and meet rows cover the whole table, because
//! neither carries a cohort.

use census_domain::model::{
    CanonicalAthlete, GradYear, ATHLETE_IDENTITY_FAMILY, COHORT_EVIDENCE_FAMILY,
    COHORT_IDENTITY_CONFIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY, CONTACT_CONFLICT_FAMILY,
    SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY, UNRESOLVED_VENUE_FAMILY,
    WITHHELD_MAILBOX_FAMILY,
};
use std::collections::HashMap;

use crate::report::ReportResult;
use crate::workbook::cells::{cell, row, Cell};
use census_store::Store;

use super::{school_name_index, Family, QueueRow, StoreRows};

mod conflicts;
mod review;

use conflicts::{athlete_identity, cohort_evidence, contact_conflicts, school_identity};
use review::{
    cohort_unverified, low_confidence, unresolved_schools, unresolved_venues, withheld_mailboxes,
};

/// Widths for a queue sheet.
pub(super) const QUEUE_WIDTHS: [u16; 4] = [30, 16, 34, 96];

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
pub(super) const WITHHELD_MAILBOX: &str = WITHHELD_MAILBOX_FAMILY;
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
        withheld_mailboxes(rows, names),
        unresolved_venues(&rows.meets),
        unresolved_schools(&rows.schools),
    ]
}

/// A queue sheet: the family counts first, then the shared header and every retained row.
pub(super) fn queue_sheet(families: &[Family], counts_label: &str) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!(Cell::text(counts_label), "Findings")];
    for family in families {
        cells.push(row!(
            Cell::text(family.label),
            Cell::number(family.findings)?,
        ));
    }
    cells.push(row!());
    cells.push(row!("Reason", "Subject id", "Subject", "Detail"));
    for family in families {
        for retained in &family.rows {
            cells.push(row!(
                Cell::text(family.label),
                Cell::text(&retained.subject_id),
                Cell::text(&retained.subject),
                Cell::text(&retained.detail),
            ));
        }
    }
    Ok(cells)
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
pub(crate) fn retained_records(store: &Store) -> ReportResult<RetainedRecords> {
    let rows = StoreRows::read(store)?;
    let names = school_name_index(&rows.schools);
    Ok(RetainedRecords {
        conflicts: labelled(conflict_families(&rows, &names)),
        reviews: labelled(review_families(&rows, &names)),
    })
}

/// The retained conflicts and reviews, each row paired with the family that produced it.
#[derive(Debug, Default)]
pub(crate) struct RetainedRecords {
    pub(crate) conflicts: Vec<(&'static str, QueueRow)>,
    pub(crate) reviews: Vec<(&'static str, QueueRow)>,
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
