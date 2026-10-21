use super::fixture::{row_with, shared_string_workbook, sheet_with, workbook, write};
use crate::report::CorruptionClass;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::read::tests::TestResult;
use crate::workbook::verify::read::{
    Book, Budget, Readback, EXCEL_COLUMNS_PER_SHEET, EXCEL_ROWS_PER_SHEET,
};

const READ_BUDGET: Budget = Budget {
    rows: EXCEL_ROWS_PER_SHEET,
    columns: EXCEL_COLUMNS_PER_SHEET,
};

const DUPLICATED_A1: &str = "<c r=\"A1\"><v>1</v></c><c r=\"A1\"><v>2</v></c>";
const REVISITED_A1: &str = "<row r=\"1\"><c r=\"A1\"><v>1</v></c></row><row r=\"2\"><c r=\"B2\"><v>2</v></c></row><row r=\"1\"><c r=\"A1\"><v>3</v></c></row>";

fn duplicate_cases() -> [(&'static str, String); 2] {
    [
        ("adjacent", row_with(DUPLICATED_A1)),
        ("revisited", sheet_with("A1:B2", REVISITED_A1)),
    ]
}

#[test]
fn inline_profile_refuses_duplicated_a1_cells() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (label, sheet) in duplicate_cases() {
        let path = write(
            directory.path(),
            &format!("duplicate-inline-{label}.xlsx"),
            &workbook(&sheet),
        )?;
        let mut book = Book::open(&path)?;
        check!(eq; book.readback(), Readback::InlineProfile, "{label}");
        let error = book
            .read("Sheet1", READ_BUDGET, |_| {})
            .err()
            .ok_or_else(|| format!("{label}: the inline profile accepted duplicated A1 cells"))?;
        check!(
            eq;
            error.corruption(),
            Some(CorruptionClass::ArtifactCorrupt),
            "{label}: {error}"
        );
    }
    Ok(())
}

#[test]
fn legacy_shared_string_lane_refuses_duplicated_a1_cells() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (label, sheet) in duplicate_cases() {
        let path = write(
            directory.path(),
            &format!("duplicate-shared-{label}.xlsx"),
            &shared_string_workbook(&sheet),
        )?;
        let mut book = Book::open(&path)?;
        check!(eq; book.readback(), Readback::LegacySharedStrings, "{label}");
        let error = book
            .read("Sheet1", READ_BUDGET, |_| {})
            .err()
            .ok_or_else(|| {
                format!("{label}: the shared-string lane accepted duplicated A1 cells")
            })?;
        check!(
            eq;
            error.corruption(),
            Some(CorruptionClass::ArtifactCorrupt),
            "{label}: {error}"
        );
    }
    Ok(())
}

#[test]
fn inline_profile_reads_exact_control_values() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = sheet_with(
        "A1:B1",
        "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>inline control</t></is></c><c r=\"B1\"><v>7.5</v></c></row>",
    );
    let path = write(directory.path(), "inline-control.xlsx", &workbook(&sheet))?;
    let mut book = Book::open(&path)?;
    check!(eq; book.readback(), Readback::InlineProfile);
    let mut values = Vec::new();
    let shape = book.read("Sheet1", READ_BUDGET, |row| {
        values.push((row.get(0).cloned(), row.get(1).cloned()));
    })?;
    let shape = shape.ok_or("Sheet1 is missing from the workbook")?;
    check!(eq; shape.rows, 1);
    check!(
        eq;
        values,
        vec![(
            Some(Value::text("inline control")),
            Some(Value::Number(7.5))
        )]
    );
    Ok(())
}

#[test]
fn legacy_shared_string_lane_reads_exact_control_values() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = write(
        directory.path(),
        "shared-control.xlsx",
        &shared_string_workbook(&row_with("<c r=\"A1\" t=\"s\"><v>0</v></c>")),
    )?;
    let mut book = Book::open(&path)?;
    check!(eq; book.readback(), Readback::LegacySharedStrings);
    let mut values = Vec::new();
    let shape = book.read("Sheet1", READ_BUDGET, |row| {
        values.push(row.get(0).cloned());
    })?;
    let shape = shape.ok_or("Sheet1 is missing from the workbook")?;
    check!(eq; shape.rows, 1);
    check!(eq; values, vec![Some(Value::text("=2+2"))]);
    Ok(())
}
