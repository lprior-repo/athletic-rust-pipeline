use super::fixture::{workbook, write};
use crate::report::CorruptionClass;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::read::tests::TestResult;
use crate::workbook::verify::read::{Book, Budget, EXCEL_COLUMNS_PER_SHEET, EXCEL_ROWS_PER_SHEET};

const READ_BUDGET: Budget = Budget {
    rows: EXCEL_ROWS_PER_SHEET,
    columns: EXCEL_COLUMNS_PER_SHEET,
};

#[test]
fn inline_profile_refuses_cell_directly_under_worksheet() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1\"/><c r=\"A1\"><v>1</v></c></worksheet>";
    let path = write(directory.path(), "cell-at-worksheet.xlsx", &workbook(sheet))?;
    let mut book = Book::open(&path)?;
    let error = book
        .read("Sheet1", READ_BUDGET, |_| {})
        .err()
        .ok_or("cell directly under worksheet was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn inline_profile_refuses_cell_directly_under_sheet_data() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1\"/><sheetData><c r=\"A1\"><v>1</v></c></sheetData></worksheet>";
    let path = write(
        directory.path(),
        "cell-at-sheet-data.xlsx",
        &workbook(sheet),
    )?;
    let mut book = Book::open(&path)?;
    let error = book
        .read("Sheet1", READ_BUDGET, |_| {})
        .err()
        .ok_or("cell directly under sheetData was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn inline_profile_refuses_cell_directly_under_ext_lst() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1\"/><extLst><c r=\"A1\"><v>1</v></c></extLst></worksheet>";
    let path = write(directory.path(), "cell-at-ext-lst.xlsx", &workbook(sheet))?;
    let mut book = Book::open(&path)?;
    let error = book
        .read("Sheet1", READ_BUDGET, |_| {})
        .err()
        .ok_or("cell directly under extLst was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn inline_profile_refuses_foreign_prefix_cell() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:foreign=\"http://example.com\"><dimension ref=\"A1\"/><sheetData><row r=\"1\"><foreign:c r=\"A1\"><foreign:v>1</foreign:v></foreign:c></row></sheetData></worksheet>";
    let path = write(
        directory.path(),
        "foreign-prefix-cell.xlsx",
        &workbook(sheet),
    )?;
    let mut book = Book::open(&path)?;
    let error = book
        .read("Sheet1", READ_BUDGET, |_| {})
        .err()
        .ok_or("cell with foreign prefix was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn inline_profile_refuses_foreign_default_namespace() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = "<worksheet xmlns=\"http://example.com/foreign\"><dimension ref=\"A1\"/><sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData></worksheet>";
    let path = write(
        directory.path(),
        "foreign-default-ns.xlsx",
        &workbook(sheet),
    )?;
    let mut book = Book::open(&path)?;
    let error = book
        .read("Sheet1", READ_BUDGET, |_| {})
        .err()
        .ok_or("worksheet with foreign default namespace was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn inline_profile_accepts_standard_default_namespace() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1\"/><sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row></sheetData></worksheet>";
    let path = write(
        directory.path(),
        "standard-namespace.xlsx",
        &workbook(sheet),
    )?;
    let mut book = Book::open(&path)?;
    let mut values = Vec::new();
    let shape = book
        .read("Sheet1", READ_BUDGET, |row| {
            values.push(row.get(0).cloned())
        })?
        .ok_or("Sheet1 is missing")?;
    check!(eq; shape.rows, 1);
    check!(eq; values, vec![Some(Value::Number(1.0))]);
    Ok(())
}
