use super::super::tests::{
    accepted_by_all, alter_sheet, copy_bundle, publication, recapture, replace_cell, TestResult,
};
use super::fixture::{extra_page, partition, rename};
use crate::report::{CorruptionClass, ReportError, ReportResult};
use crate::workbook::verify::{verify_frozen, VerifiedWorkbook};
use calamine::Reader;
use std::path::Path;

#[test]
fn frozen_verification_reads_real_two_and_three_page_projections() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    let original = verify_frozen(&good, &dataset, &options)?;
    check!(eq; original.mapped_athletes, 1);
    let path = copy_bundle(&good, &directory.path().join("paged"))?;
    check!(eq; partition(&path, "Coaches", 1)?, 2);
    check!(eq; partition(&path, "Schools", 1)?, 2);
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
    let metrics = source.worksheet_range("Run Metrics")?;
    let data_rows = metrics.height().checked_sub(1).ok_or("metrics header")?;
    check!(data_rows >= 3, "fixture needs three metadata data rows");
    check!(eq; partition(&path, "Run Metrics", data_rows.div_ceil(3))?, 3);
    let actual = verify_frozen(&path, &dataset, &options)?;
    check!(eq; actual.rows, original.rows);
    check!(eq; actual.mapped_athletes, 1);
    check!(eq; actual.sheets, original.sheets.checked_add(4).ok_or("sheet overflow")?);
    recapture(&path, &dataset, &options)?;
    accepted_by_all(&path, &dataset, &options)?;
    Ok(())
}

#[test]
fn frozen_verification_refuses_mutated_late_pages_and_family_inventory() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    for mode in [
        "missing-base",
        "gap",
        "alias",
        "header",
        "extra-header",
        "late-cell",
        "extra-row",
        "extra-part",
        "empty-part",
    ] {
        let path = copy_bundle(&good, &directory.path().join(mode))?;
        check!(eq; partition(&path, "Coaches", 1)?, 2);
        let before = verify_frozen(&path, &dataset, &options)?;
        check!(eq; before.mapped_athletes, 1);
        mutate(&path, mode)?;
        let class = if mode == "late-cell" {
            CorruptionClass::SemanticMismatch
        } else {
            CorruptionClass::ArtifactCorrupt
        };
        rejected(verify_frozen(&path, &dataset, &options), &path, class)?;
    }
    Ok(())
}

#[test]
fn inventory_never_exempts_unknown_sheets_or_unused_performance_parts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    for name in [
        "Unrecognized_002",
        "Performances_001_002",
        "Performances_002",
    ] {
        let path = copy_bundle(&good, &directory.path().join(name))?;
        check!(eq; partition(&path, "Coaches", 1)?, 2);
        check!(eq; verify_frozen(&path, &dataset, &options)?.mapped_athletes, 1);
        extra_page(&path, name, false)?;
        rejected(
            verify_frozen(&path, &dataset, &options),
            &path,
            CorruptionClass::SemanticMismatch,
        )?;
    }
    Ok(())
}

#[test]
fn frozen_verification_checks_every_row_of_a_third_metadata_page() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    let path = copy_bundle(&good, &directory.path().join("late-metrics"))?;
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
    let rows = source
        .worksheet_range("Run Metrics")?
        .height()
        .checked_sub(1)
        .ok_or("metrics header")?;
    check!(eq; partition(&path, "Run Metrics", rows.div_ceil(3))?, 3);
    check!(eq; verify_frozen(&path, &dataset, &options)?.mapped_athletes, 1);
    alter_sheet(&path, "Run Metrics_003", |range| {
        let (row, column) = range.end().ok_or("third page last cell")?;
        check!(row >= 1, "third page must contain a real record");
        replace_cell(range, (row, column), "Late forged metadata");
        Ok(())
    })?;
    rejected(
        verify_frozen(&path, &dataset, &options),
        &path,
        CorruptionClass::SemanticMismatch,
    )?;
    Ok(())
}

fn mutate(path: &Path, mode: &str) -> TestResult {
    match mode {
        "missing-base" => rename(path, "Coaches", "Detached Coaches"),
        "gap" => rename(path, "Coaches_002", "Coaches_003"),
        "alias" => rename(path, "Coaches_002", "Coaches_02"),
        "extra-part" => extra_page(path, "Coaches_003", false),
        "empty-part" => extra_page(path, "Coaches_003", true),
        _ => alter_sheet(path, "Coaches_002", |range| {
            match mode {
                "header" => replace_cell(range, (0, 1), "Forged school header"),
                "extra-header" => replace_cell(range, (0, 29), "Hidden header"),
                "late-cell" => replace_cell(range, (1, 9), "forged@fixture.test"),
                "extra-row" => replace_cell(range, (2, 0), "Extra retained row"),
                _ => return Err(format!("unknown mutation {mode}").into()),
            }
            Ok(())
        }),
    }
}

fn rejected(
    result: ReportResult<VerifiedWorkbook>,
    path: &Path,
    class: CorruptionClass,
) -> TestResult {
    let error = result.err().ok_or("forged workbook accepted")?;
    check!(eq; error.corruption(), Some(class));
    match error {
        ReportError::ArtifactCorrupt { path: found, .. }
        | ReportError::SemanticMismatch { path: found, .. } => check!(eq; found, path),
        other => return Err(format!("unexpected refusal: {other}").into()),
    }
    Ok(())
}
