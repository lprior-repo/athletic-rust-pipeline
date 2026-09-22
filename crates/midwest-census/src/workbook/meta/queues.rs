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

use census_domain::model::{CanonicalAthlete, GradYear};
use std::collections::HashMap;

use crate::report::ReportResult;
use crate::workbook::cells::{cell, row, Cell};

use super::{Family, StoreRows};

mod conflicts;
mod review;

use conflicts::{athlete_identity, cohort_evidence, school_identity};
use review::{
    cohort_unverified, low_confidence, unresolved_schools, unresolved_venues, withheld_mailboxes,
};

/// Widths for a queue sheet.
pub(super) const QUEUE_WIDTHS: [u16; 4] = [30, 16, 34, 96];

/// Family labels, shared with the reconciliation block on `Run Metrics`.
pub(super) const COHORT_EVIDENCE: &str = "Class-of-2027 cohort evidence";
pub(super) const ATHLETE_IDENTITY: &str = "Athlete identity";
pub(super) const SCHOOL_IDENTITY: &str = "School identity";
pub(super) const COHORT_UNVERIFIED: &str = "Class-of-2027 cohort unverified";
pub(super) const LOW_CONFIDENCE: &str = "Class-of-2027 identity confidence";
pub(super) const WITHHELD_MAILBOX: &str = "Coach mailbox withheld";
pub(super) const UNRESOLVED_VENUE: &str = "Meet venue unresolved";
pub(super) const UNRESOLVED_SCHOOL: &str = "School jurisdiction unresolved";

/// Every conflict family the store retains.
pub(super) fn conflict_families(rows: &StoreRows, names: &HashMap<&str, &str>) -> Vec<Family> {
    vec![
        cohort_evidence(rows, names),
        athlete_identity(rows, names),
        school_identity(&rows.schools),
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
        cells.extend_from_slice(&family.rows);
    }
    Ok(cells)
}

/// The published cohort's athlete rows: the same class the census document counts.
fn class_of_2027(athletes: &[CanonicalAthlete]) -> impl Iterator<Item = &CanonicalAthlete> + '_ {
    athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
}

/// One queue row: the family label, the subject's id and name, and why the row is unresolved.
fn queue_row(label: &str, id: &str, subject: String, detail: String) -> Vec<Cell> {
    row!(
        Cell::text(label),
        Cell::text(id),
        Cell::text(subject),
        Cell::text(detail)
    )
}
