//! The operational sheets (§54): the school and meet inventories, the source declarations and the
//! evidence they left, the coverage table, the retained conflict and review queues, and the run's
//! own counters.
//!
//! Every row is a merged store row ([`Store::scan`]) or a counter the census document already
//! publishes; nothing is read from the materialized `out/*.jsonl` export, exactly as the census
//! document itself reads the store. The sheet that prints a count also prints the census number it
//! must equal — `Run Metrics` carries that reconciliation block — so a workbook that has drifted
//! from `report.json` says so in its own cells instead of leaving the reader to diff two artifacts.
//!
//! The row-level sheets render what the store retains rather than what is convenient to count: a
//! school another school's normalized name collides with, an athlete whose own grade observations
//! disagree, a meet whose venue was never placed, a coach whose only published address was a
//! personal mailbox and was therefore withheld. Those rows are the operator's work queue, so the
//! sheets carry the subject id of every one of them.
//!
//! This module is the entry point and the shared vocabulary: each sheet family lives beside it
//! ([`inventory`], [`coverage`], [`queues`], [`sources`], [`metrics`]) and reads the same
//! [`StoreRows`] snapshot, so the workbook scans each table once.

use crate::bests::BestResult;
use crate::report::{in_run_scope, jurisdiction_of, school_state_index, Census, ReportResult};
use census_domain::{
    model::{CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool},
    JurisdictionBucket,
};
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::cells::{write_sheet, Cell};

mod coverage;
mod inventory;
mod metrics;
pub mod queues;
mod sources;

use coverage::{coverage_sheet, COVERAGE_WIDTHS};
use inventory::{meets_sheet, schools_sheet, MEET_WIDTHS, SCHOOL_WIDTHS};
use metrics::{metrics_sheet, METRIC_WIDTHS};
use queues::{conflict_families, queue_sheet, review_families, QUEUE_WIDTHS};

/// The retained queue rows, as the store's `conflicts` and `review_cases` tables hold them.
pub use queues::retained_records;
use sources::{sources_sheet, SOURCE_WIDTHS};

/// One sheet of the workbook: the sheet name, its rows, its column widths and whether the header
/// carries an autofilter.
type Sheet = (&'static str, Vec<Vec<Cell>>, &'static [u16], bool);

/// Write §54's remaining sheets into `book`, in the order the objective lists them.
pub(super) fn write_meta_sheets(
    book: &mut Workbook,
    path: &Path,
    store: &Store,
    core: &Census,
    all_sources: &Census,
    bests: &[BestResult],
) -> ReportResult<()> {
    let rows = StoreRows::read(store)?;
    let names = school_name_index(&rows.schools);
    let conflicts = conflict_families(&rows, &names);
    let review = review_families(&rows, &names);
    let metrics = metrics_sheet(store, core, all_sources, bests, &rows, &conflicts)?;
    // One entry per sheet, in published order: the name, the rows, the column widths and whether the
    // header carries an autofilter.
    let sheets: [Sheet; 7] = [
        ("Schools", schools_sheet(&rows)?, &SCHOOL_WIDTHS, true),
        ("Meets", meets_sheet(&rows.meets), &MEET_WIDTHS, true),
        ("Sources", sources_sheet(all_sources)?, &SOURCE_WIDTHS, true),
        ("Coverage", coverage_sheet(store)?, &COVERAGE_WIDTHS, true),
        (
            "Conflicts",
            queue_sheet(&conflicts, "Retained conflicts")?,
            &QUEUE_WIDTHS,
            true,
        ),
        (
            "Review",
            queue_sheet(&review, "Retained review queue")?,
            &QUEUE_WIDTHS,
            true,
        ),
        ("Run Metrics", metrics, &METRIC_WIDTHS, false),
    ];
    for (name, cells, widths, autofilter) in sheets {
        write_sheet(book, path, name, cells, widths, autofilter)?;
    }
    Ok(())
}

/// The merged store rows every operational sheet reads, read once for the whole workbook.
struct StoreRows {
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    athletes: Vec<CanonicalAthlete>,
    coaches: Vec<CanonicalCoach>,
}
impl StoreRows {
    /// Read the four entity tables the operational sheets render, scoped to the run's
    /// jurisdictions (`CENSUS_SCOPE` + unplaced) so the workbook reconciliation block
    /// (ADR-009) matches the census totals computed by the same predicate.
    fn read(store: &Store) -> ReportResult<Self> {
        let mut schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
        schools.retain(|s| in_run_scope(JurisdictionBucket::from(s.state)));
        let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
        meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
        let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
        // Athlete jurisdiction comes from school state (the report's placement rule).
        let school_state = school_state_index(&schools);
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        let mut coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        // Coach jurisdiction also comes from school state.
        coaches.retain(|c| in_run_scope(jurisdiction_of(&school_state, c.school.as_str())));
        Ok(Self {
            schools,
            meets,
            athletes,
            coaches,
        })
    }
}

/// A saturating counter bump: a tally cannot exceed the rows it was built from.
fn bump(counter: &mut usize) {
    *counter = counter.saturating_add(1);
}

/// One family of retained rows: the label the counts block prints, how many findings it holds (a
/// group-style family renders several rows per finding), and the rows themselves.
/// One retained row: the subject it names and why the row is unresolved.
///
/// The family that owns the row carries the label, so the same value renders in a sheet and lands in
/// the store's `conflicts`/`review_cases` tables without a second copy of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRow {
    pub subject_id: String,
    pub subject: String,
    pub detail: String,
}

struct Family {
    label: &'static str,
    findings: usize,
    rows: Vec<QueueRow>,
}

impl Family {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            findings: 0,
            rows: Vec::new(),
        }
    }

    /// Record one finding that renders as exactly one row.
    fn push(&mut self, row: QueueRow) {
        self.findings = self.findings.saturating_add(1);
        self.rows.push(row);
    }

    /// Record one finding that renders as several rows.
    fn group(&mut self, rows: Vec<QueueRow>) {
        self.findings = self.findings.saturating_add(1);
        self.rows.extend(rows);
    }
}

/// School id -> the school's display name, for subject lines that name a school.
fn school_name_index(schools: &[CanonicalSchool]) -> HashMap<&str, &str> {
    schools
        .iter()
        .map(|school| (school.id.as_str(), school.name.as_str()))
        .collect()
}

/// The display name behind a school id, when the school table holds one.
fn school_of<'a>(names: &HashMap<&'a str, &'a str>, school: &str) -> Option<&'a str> {
    names.get(school).copied()
}

/// A row subject: the name, with the school it belongs to when the table knows it.
fn subject_of(name: &str, school: Option<&str>) -> String {
    match school {
        Some(school) => format!("{name} ({school})"),
        None => name.to_string(),
    }
}

/// A count table's entries by descending count, then key: the reading order every block uses, so two
/// sheets never disagree about which source leads.
fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(&String, &usize)> {
    let mut ordered: Vec<(&String, &usize)> = counts.iter().collect();
    ordered.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    ordered
}

#[cfg(test)]
mod tests;
