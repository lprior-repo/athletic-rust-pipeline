//! The operational sheets (§54): the meet inventory, the source declarations and the evidence they
//! left, the coverage table, the retained data-quality queues, and the run's own counters.
//!
//! Every row is a merged store row ([`Store::scan`]) or a counter the census document already
//! publishes; nothing is read from the materialized `out/*.jsonl` export, exactly as the census
//! document itself reads the store. The sheet that prints a count also prints the census number it
//! must equal — `Run Metrics` carries that reconciliation block — so a workbook that has drifted
//! from `report.json` says so in its own cells instead of leaving the reader to diff two artifacts.
//!
//! The row-level sheets render what the store retains rather than what is convenient to count: a
//! school another school's normalized name collides with, an athlete whose own grade observations
//! disagree, or a meet whose venue was never placed. Those rows are the operator's work queue, so
//! the sheets carry the subject id of every one of them.
//!
//! This module is the entry point and the shared vocabulary: each sheet family lives beside it
//! (`inventory`, `coverage`, [`queues`], `sources`, `metrics`) and reads the same `StoreRows` snapshot,
//! so the workbook scans each table once.

use crate::bests::BestResult;
use crate::report::{
    exclude_out_of_scope, in_run_scope, jurisdiction_of, retain_core, school_state_index, Census,
    ReportResult, Scope,
};
use census_domain::{
    model::{
        CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, ReviewVerdictRecord,
    },
    JurisdictionBucket,
};
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::cells::{write_sheet, Cell};
use super::performances::PerformanceSheetPopulation;

mod coverage;
mod inventory;
mod metrics;
pub mod queues;
mod schools;
mod sources;

use coverage::{coverage_sheet, COVERAGE_WIDTHS};
use inventory::{meets_sheet, MEET_WIDTHS};
use metrics::{metrics_sheet, METRIC_WIDTHS};
use queues::{
    conflict_families, conflicts_sheet, review_families, review_sheet, CONFLICT_WIDTHS,
    REVIEW_WIDTHS,
};

/// The retained queue rows, as the store's `conflicts` and `review_cases` tables hold them.
pub use queues::retained_records;
use schools::{schools_sheet, SCHOOL_WIDTHS};
use sources::{sources_sheet, SOURCE_WIDTHS};

/// One sheet of the workbook: the sheet name, its rows, its column widths and whether the header
/// carries an autofilter.
type Sheet = (&'static str, Vec<Vec<Cell>>, &'static [u16], bool);

/// The published facts every §54 sheet's counters read, gathered once per run so the sheet builders
/// take one run instead of a loose tail of parts.
pub(super) struct RunFacts<'a> {
    pub(super) store: &'a Store,
    pub(super) core: &'a Census,
    pub(super) all_sources: &'a Census,
    pub(super) bests: &'a [BestResult],
    pub(super) scope: Scope,
    pub(super) perf_population: PerformanceSheetPopulation,
}

/// Write §54's remaining sheets into `book`, in the frozen order.
pub(super) fn write_meta_sheets(
    book: &mut Workbook,
    path: &Path,
    facts: RunFacts<'_>,
) -> ReportResult<()> {
    let rows = StoreRows::read(facts.store, facts.scope)?;
    let names = school_name_index(&rows.schools);
    let conflicts = conflict_families(&rows, &names);
    let review = review_families(&rows, &names);
    let metrics = metrics_sheet(&facts, &rows, &conflicts)?;
    let sheets: [Sheet; 7] = [
        (
            "Schools",
            schools_sheet(&rows.schools)?,
            &SCHOOL_WIDTHS,
            true,
        ),
        ("Meets", meets_sheet(&rows.meets), &MEET_WIDTHS, true),
        (
            "Sources",
            sources_sheet(facts.all_sources)?,
            &SOURCE_WIDTHS,
            true,
        ),
        (
            "Coverage",
            coverage_sheet(facts.store)?,
            &COVERAGE_WIDTHS,
            true,
        ),
        (
            "Conflicts",
            conflicts_sheet(&conflicts, &rows),
            &CONFLICT_WIDTHS,
            true,
        ),
        ("Review", review_sheet(&review, &rows), &REVIEW_WIDTHS, true),
        ("Run Metrics", metrics, &METRIC_WIDTHS, false),
    ];
    for (name, cells, widths, autofilter) in sheets {
        write_sheet(book, path, name, cells, widths, autofilter)?;
    }
    Ok(())
}

/// The merged store rows every operational sheet reads, read once for the workbook.
struct StoreRows {
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    athletes: Vec<CanonicalAthlete>,
    coaches: Vec<CanonicalCoach>,
    verdicts: Vec<ReviewVerdictRecord>,
}
impl StoreRows {
    /// Read the entity tables scoped to the run's jurisdictions (`CENSUS_SCOPE` + unplaced) so
    /// the reconciliation block matches the run's published scope. Verdicts are an extra read from
    /// their durable table, preserving store order for the review sheet's verdict rows.
    fn read(store: &Store, scope: Scope) -> ReportResult<Self> {
        let mut schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
        // The placement index carries the excluded school rows too, so an athlete or coach whose
        // school the run scope leaves out is placed by that school's jurisdiction and excluded with
        // it, instead of reading as unplaced and inflating the counts this block reconciles.
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
        meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
        let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
        // Athlete jurisdiction comes from school state (the report's placement rule).
        let mut school_state = school_state_index(&schools);
        school_state.extend(school_state_index(&outside_schools));
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        if scope == Scope::Core {
            retain_core(&mut meets);
            retain_core(&mut athletes);
        }
        let mut coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        // Coach jurisdiction also comes from school state.
        coaches.retain(|c| in_run_scope(jurisdiction_of(&school_state, c.school.as_str())));
        let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts)?;
        Ok(Self {
            schools,
            meets,
            athletes,
            coaches,
            verdicts,
        })
    }
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
