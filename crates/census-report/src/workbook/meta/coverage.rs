use crate::report::{coverage_report, CoverageReport, JurisdictionCoverage, ReportResult};
use census_domain::model::GradYear;
use census_store::Store;
use std::collections::BTreeMap;

use crate::workbook::cells::{row, Cell};

use super::sorted_counts;

pub(super) const COVERAGE_WIDTHS: [u16; 29] = [
    12, 9, 12, 11, 12, 9, 8, 8, 11, 10, 11, 9, 8, 7, 12, 13, 11, 11, 11, 12, 11, 9, 12, 12, 12, 12,
    9, 12, 32,
];

#[derive(Clone, Copy)]
enum Column {
    Count(fn(&JurisdictionCoverage) -> usize),
    Percent(fn(&JurisdictionCoverage) -> u32),
}

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
