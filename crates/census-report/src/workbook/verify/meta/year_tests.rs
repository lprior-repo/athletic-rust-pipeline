use super::{expected_sheets, Expect};
use crate::export::ExportDataset;
use crate::workbook::verify::expectations::Expectations;
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{COHORT_EVIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY};
use std::collections::BTreeSet;
use std::path::Path;

mod fixture;
mod tamper;

use fixture::{fixture, options};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SELECTIONS: [(Option<i16>, &str, &str); 3] = [
    (Some(2027), "1", "class of 2027"),
    (Some(2028), "2", "class of 2028"),
    (None, "7", "all graduation years"),
];

#[test]
fn scalar_year_writer_coverage_follows_the_workbook_selection() -> TestResult {
    let fixture = fixture()?;
    for (year, count, _) in SELECTIONS {
        let (path, _) = fixture.publish(year)?;
        let rows = sheet(&path, "Coverage")?;
        check!(eq; field(&rows, "WI", 3)?, count);
    }
    Ok(())
}

#[test]
fn scalar_year_writer_review_keeps_requested_unresolved_candidates() -> TestResult {
    let fixture = fixture()?;
    for (year, _, _) in SELECTIONS {
        let (path, _) = fixture.publish(year)?;
        let rows = sheet(&path, "Review")?;
        check!(eq; subjects(&rows, COHORT_UNVERIFIED_FAMILY)?, ids(fixture.review_ids(year)));
    }
    Ok(())
}

#[test]
fn scalar_year_writer_conflicts_keep_the_requested_cohort() -> TestResult {
    let fixture = fixture()?;
    for (year, _, _) in SELECTIONS {
        let (path, _) = fixture.publish(year)?;
        let rows = sheet(&path, "Conflicts")?;
        check!(eq; subjects(&rows, COHORT_EVIDENCE_FAMILY)?, ids(fixture.conflict_ids(year)));
    }
    Ok(())
}

#[test]
fn scalar_year_writer_metrics_distinguish_selection_and_legacy_census() -> TestResult {
    let fixture = fixture()?;
    for (year, count, label) in SELECTIONS {
        let (path, _) = fixture.publish(year)?;
        assert_metrics(&sheet(&path, "Run Metrics")?, count, label)?;
    }
    Ok(())
}

#[test]
fn scalar_year_readback_coverage_uses_the_requested_year() -> TestResult {
    let fixture = fixture()?;
    let dataset = ExportDataset::load(&fixture.store)?;
    for (year, count, _) in SELECTIONS {
        let rows = expected(&dataset, year, "Coverage")?;
        check!(eq; field(&rows, "WI", 3)?, count);
    }
    Ok(())
}

#[test]
fn scalar_year_readback_review_preserves_unresolved_selected_candidates() -> TestResult {
    let fixture = fixture()?;
    let dataset = ExportDataset::load(&fixture.store)?;
    for (year, _, _) in SELECTIONS {
        let rows = expected(&dataset, year, "Review")?;
        check!(eq; subjects(&rows, COHORT_UNVERIFIED_FAMILY)?, ids(fixture.review_ids(year)));
    }
    Ok(())
}

#[test]
fn scalar_year_readback_conflicts_follow_the_requested_cohort() -> TestResult {
    let fixture = fixture()?;
    let dataset = ExportDataset::load(&fixture.store)?;
    for (year, _, _) in SELECTIONS {
        let rows = expected(&dataset, year, "Conflicts")?;
        check!(eq; subjects(&rows, COHORT_EVIDENCE_FAMILY)?, ids(fixture.conflict_ids(year)));
    }
    Ok(())
}

#[test]
fn scalar_year_readback_metrics_name_the_selected_and_legacy_cohorts() -> TestResult {
    let fixture = fixture()?;
    let dataset = ExportDataset::load(&fixture.store)?;
    for (year, count, label) in SELECTIONS {
        assert_metrics(&expected(&dataset, year, "Run Metrics")?, count, label)?;
    }
    Ok(())
}

#[test]
fn scalar_year_retained_records_preserve_the_explicit_legacy_cohort() -> TestResult {
    let fixture = fixture()?;
    let dataset = ExportDataset::load(&fixture.store)?;
    let selection = options(Some(2028))?;
    let retained = crate::workbook::retained_records(
        &dataset,
        selection.school_year,
        &selection.jurisdictions,
    )?;
    let observed: BTreeSet<_> = retained
        .reviews
        .iter()
        .filter(|(family, _)| *family == COHORT_UNVERIFIED_FAMILY)
        .map(|(_, row)| row.subject_id.clone())
        .collect();
    check!(eq; observed, ids(fixture.review_ids(Some(2027))));
    Ok(())
}

fn expected(
    dataset: &ExportDataset,
    year: Option<i16>,
    name: &str,
) -> TestResult<Vec<Vec<String>>> {
    let expectations = Expectations::of(dataset, &options(year)?)?;
    let (_, rows) = expected_sheets(&expectations)?
        .into_iter()
        .find(|(sheet, _)| *sheet == name)
        .ok_or("expected metadata sheet")?;
    Ok(rows
        .into_iter()
        .map(|row| row.into_iter().map(expect_text).collect())
        .collect())
}

fn expect_text(cell: Expect) -> String {
    match cell {
        Expect::Text(value) => value,
        Expect::Number(value) => value.to_string(),
        Expect::Empty => String::new(),
    }
}

fn sheet(path: &Path, name: &str) -> TestResult<Vec<Vec<String>>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let range = workbook.worksheet_range(name)?;
    Ok(range
        .rows()
        .map(|row| row.iter().map(ToString::to_string).collect())
        .collect())
}

fn field<'a>(rows: &'a [Vec<String>], label: &str, column: usize) -> TestResult<&'a str> {
    rows.iter()
        .find(|row| row.first().is_some_and(|cell| cell == label))
        .and_then(|row| row.get(column))
        .map(String::as_str)
        .ok_or_else(|| format!("missing {label} column {column}").into())
}

fn subjects(rows: &[Vec<String>], family: &str) -> TestResult<BTreeSet<String>> {
    rows.iter()
        .filter(|row| row.first().is_some_and(|cell| cell == family))
        .map(|row| {
            row.get(2)
                .cloned()
                .ok_or_else(|| "missing subject id".into())
        })
        .collect()
}

fn ids(values: Vec<String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

fn assert_metrics(rows: &[Vec<String>], count: &str, label: &str) -> TestResult {
    check!(eq; field(rows, "Workbook cohort", 1)?, label);
    check!(eq; field(rows, "Recruiting athletes", 1)?, count);
    check!(eq; field(rows, "Cohort behind census counters", 1)?, "class of 2027");
    check!(eq; field(rows, "Class of 2027", 1)?, "3");
    check!(eq; field(rows, "Class of 2027", 3)?, "3");
    check!(eq; field(rows, "Class-of-2027 athletes", 1)?, "3");
    check!(eq; field(rows, "Class-of-2027 athletes", 2)?, "3");
    check!(eq; field(rows, "Class-of-2027 athletes", 3)?, "reconciled");
    check!(!rows.iter().any(|row| row
        .first()
        .is_some_and(|cell| cell == "Cohort behind the counters")));
    Ok(())
}
