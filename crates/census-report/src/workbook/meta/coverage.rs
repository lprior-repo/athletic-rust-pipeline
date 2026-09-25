//! The coverage sheet: §49's per-jurisdiction denominators, the gap register, and how far each
//! source reached.
//!
//! The numbers come from `report::coverage_report`, the same pass the report's coverage output
//! prints, and this module renders it rather than recounting anything: the workbook therefore cannot
//! disagree with the JSON. The cohort is the class of 2027 — the counters the census document
//! publishes describe that cohort, and a coverage table measured against a different one would not
//! reconcile with the rest of the workbook.

use crate::report::{coverage_report, CoverageReport, JurisdictionCoverage, ReportResult};
use census_domain::model::GradYear;
use census_store::Store;
use std::collections::BTreeMap;

use crate::workbook::cells::{row, Cell};

use super::sorted_counts;

/// Widths for the jurisdiction table.
pub(super) const COVERAGE_WIDTHS: [u16; 29] = [
    12, 9, 12, 11, 12, 9, 8, 8, 11, 10, 11, 9, 8, 7, 12, 13, 11, 11, 11, 12, 11, 9, 12, 12, 12, 12,
    9, 12, 32,
];

/// One numeric column of the jurisdiction table: a count, or a percentage the coverage pass already
/// reduced to a whole number.
#[derive(Clone, Copy)]
enum Column {
    Count(fn(&JurisdictionCoverage) -> usize),
    Percent(fn(&JurisdictionCoverage) -> u32),
}

/// §49's columns, in published order: the denominators a reader checks before quoting a total.
const COLUMNS: [(&str, Column); 27] = [
    ("Schools", Column::Count(|row| row.schools)),
    (
        "Schools with athletes",
        Column::Count(|row| row.schools_with_athletes),
    ),
    ("Athletes", Column::Count(|row| row.athletes)),
    ("Athletes (core)", Column::Count(|row| row.athletes_core)),
    ("Core share %", Column::Percent(|row| row.core_share_pct)),
    ("Boys", Column::Count(|row| row.boys)),
    ("Girls", Column::Count(|row| row.girls)),
    ("Unknown gender", Column::Count(|row| row.unknown_gender)),
    ("Grad verified", Column::Count(|row| row.grad_verified)),
    ("Grad unresolved", Column::Count(|row| row.grad_unresolved)),
    ("Outdoor", Column::Count(|row| row.outdoor_track)),
    ("Indoor", Column::Count(|row| row.indoor_track)),
    ("XC", Column::Count(|row| row.cross_country)),
    (
        "With performance",
        Column::Count(|row| row.with_performance),
    ),
    (
        "With comparable mark",
        Column::Count(|row| row.with_comparable_mark),
    ),
    ("Multisource", Column::Count(|row| row.multisource)),
    (
        "Identity conflicts",
        Column::Count(|row| row.identity_conflicts),
    ),
    ("Profile URL", Column::Count(|row| row.with_profile_url)),
    (
        "Athletic.net URL",
        Column::Count(|row| row.with_athletic_net_url),
    ),
    ("MileSplit URL", Column::Count(|row| row.with_milesplit_url)),
    ("Coaches", Column::Count(|row| row.coaches)),
    (
        "Coaches with email",
        Column::Count(|row| row.coaches_with_email),
    ),
    (
        "Schools with TF coach",
        Column::Count(|row| row.schools_with_tf_coach),
    ),
    (
        "Schools with XC coach",
        Column::Count(|row| row.schools_with_xc_coach),
    ),
    (
        "Schools with coach email",
        Column::Count(|row| row.schools_with_coach_email),
    ),
    ("Meets", Column::Count(|row| row.meets)),
    ("Performances", Column::Count(|row| row.performances)),
];

/// The jurisdiction table, the gap register, the per-source reach, and the pass's own notes.
pub(super) fn coverage_sheet(store: &Store) -> ReportResult<Vec<Vec<Cell>>> {
    let report = coverage_report(store, Some(GradYear::CO2027.get()))?;
    let mut cells = jurisdiction_table(&report)?;
    cells.push(row!());
    cells.extend(gap_table(&report)?);
    cells.push(row!());
    cells.extend(source_reach(&report)?);
    cells.push(row!());
    cells.extend(notes(&report)?);
    Ok(cells)
}

/// One row per configured jurisdiction — every jurisdiction the pass was asked about is present even
/// when it holds no rows, so a missing jurisdiction is a bug, never a silently absent line.
fn jurisdiction_table(report: &CoverageReport) -> ReportResult<Vec<Vec<Cell>>> {
    let mut headers = vec![Cell::text("Jurisdiction")];
    for (header, _) in COLUMNS {
        headers.push(Cell::text(header));
    }
    let mut cells = vec![headers];
    for jurisdiction in &report.jurisdictions {
        cells.push(jurisdiction_row(jurisdiction)?);
    }
    Ok(cells)
}

/// One jurisdiction's row, in `COLUMNS` order.
fn jurisdiction_row(jurisdiction: &JurisdictionCoverage) -> ReportResult<Vec<Cell>> {
    let mut cells = vec![Cell::text(jurisdiction.jurisdiction.code())];
    for (_, column) in COLUMNS {
        cells.push(match column {
            Column::Count(read) => Cell::number(read(jurisdiction))?,
            Column::Percent(read) => Cell::Number(f64::from(read(jurisdiction))),
        });
    }
    Ok(cells)
}

/// The gap register (§47): one row per gap class per jurisdiction, with the unit the count is in.
fn gap_table(report: &CoverageReport) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Gap jurisdiction", "Gap class", "Unit", "Count")];
    for gap in &report.gaps {
        cells.push(row!(
            Cell::text(gap.jurisdiction.code()),
            Cell::text(gap.class.as_str()),
            Cell::text(gap.unit),
            Cell::number(gap.count)?,
        ));
    }
    Ok(cells)
}

/// How far each source reached: the per-jurisdiction source maps summed into one column, because
/// the jurisdiction table cannot carry a provider list and this is the level a reader compares.
fn source_reach(report: &CoverageReport) -> ReportResult<Vec<Vec<Cell>>> {
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    for jurisdiction in &report.jurisdictions {
        for (source, count) in &jurisdiction.sources {
            let entry = totals.entry(source.clone()).or_default();
            *entry = entry.saturating_add(*count);
        }
    }
    let mut cells = vec![row!("Source", "Athletes reached", "", "")];
    for (source, count) in sorted_counts(&totals) {
        cells.push(row!(
            Cell::text(source.clone()),
            Cell::number(*count)?,
            Cell::Empty,
            Cell::Empty,
        ));
    }
    Ok(cells)
}

/// The coverage pass's own provenance: the cohort it measured, the athletes it saw outside it, and
/// the notes it recorded.
fn notes(report: &CoverageReport) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Coverage note", "Value")];
    cells.push(row!(
        "Cohort",
        Cell::text(
            report
                .grad_year
                .map_or_else(|| "every athlete".to_string(), |year| year.to_string())
        )
    ));
    cells.push(row!(
        "Athletes seen outside the cohort",
        Cell::number(report.off_cohort_athletes)?
    ));
    for note in &report.notes {
        cells.push(row!("Note", Cell::text(note.clone())));
    }
    Ok(cells)
}
