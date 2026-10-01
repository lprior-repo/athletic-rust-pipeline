use crate::export::ExportDataset;
use crate::report::{coverage_report, CoverageReport, JurisdictionCoverage, ReportResult};
use census_domain::model::GradYear;
use std::collections::BTreeMap;

use super::{header, sorted_counts, Expect, Sheet};

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

pub(super) fn expected(dataset: &ExportDataset) -> ReportResult<Sheet> {
    let report = coverage_report(dataset, Some(GradYear::CO2027.get()))?;
    let mut rows = jurisdiction_table(&report)?;
    rows.push(Vec::new());
    rows.extend(gap_table(&report)?);
    rows.push(Vec::new());
    rows.extend(source_reach(&report)?);
    rows.push(Vec::new());
    rows.extend(notes(&report)?);
    Ok(("Coverage", rows))
}

fn jurisdiction_table(report: &CoverageReport) -> ReportResult<Vec<Vec<Expect>>> {
    let mut head = vec![Expect::text("Jurisdiction")];
    head.extend(COLUMNS.iter().map(|(name, _)| Expect::text(*name)));
    let mut rows = vec![head];
    for jurisdiction in &report.jurisdictions {
        rows.push(jurisdiction_row(jurisdiction)?);
    }
    Ok(rows)
}

fn jurisdiction_row(jurisdiction: &JurisdictionCoverage) -> ReportResult<Vec<Expect>> {
    let mut row = vec![Expect::text(jurisdiction.jurisdiction.code())];
    for (_, column) in COLUMNS {
        row.push(match column {
            Column::Count(read) => Expect::count(read(jurisdiction))?,
            Column::Percent(read) => Expect::Number(f64::from(read(jurisdiction))),
        });
    }
    Ok(row)
}

fn gap_table(report: &CoverageReport) -> ReportResult<Vec<Vec<Expect>>> {
    let mut rows = vec![header(&["Gap jurisdiction", "Gap class", "Unit", "Count"])];
    for gap in &report.gaps {
        rows.push(vec![
            Expect::text(gap.jurisdiction.code()),
            Expect::text(gap.class.as_str()),
            Expect::text(gap.unit),
            Expect::count(gap.count)?,
        ]);
    }
    Ok(rows)
}

fn source_reach(report: &CoverageReport) -> ReportResult<Vec<Vec<Expect>>> {
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    for jurisdiction in &report.jurisdictions {
        for (source, count) in &jurisdiction.sources {
            let entry = totals.entry(source.clone()).or_default();
            *entry = entry.saturating_add(*count);
        }
    }
    let mut rows = vec![header(&["Source", "Athletes reached", "", ""])];
    for (source, count) in sorted_counts(&totals) {
        rows.push(vec![
            Expect::text(source.as_str()),
            Expect::count(*count)?,
            Expect::Empty,
            Expect::Empty,
        ]);
    }
    Ok(rows)
}

fn notes(report: &CoverageReport) -> ReportResult<Vec<Vec<Expect>>> {
    let mut rows = vec![header(&["Coverage note", "Value"])];
    let cohort = report
        .grad_year
        .map_or_else(|| "every athlete".to_string(), |year| year.to_string());
    rows.push(vec![Expect::text("Cohort"), Expect::text(cohort)]);
    rows.push(vec![
        Expect::text("Athletes seen outside the cohort"),
        Expect::count(report.off_cohort_athletes)?,
    ]);
    for note in &report.notes {
        rows.push(vec![Expect::text("Note"), Expect::text(note.as_str())]);
    }
    Ok(rows)
}
