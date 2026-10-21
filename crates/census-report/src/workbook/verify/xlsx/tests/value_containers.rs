use super::{failure, row_with, walk, workbook, write, Archive, Profile, TestResult, Value};
use crate::report::CorruptionClass;

#[test]
fn xlsx_profile_refuses_sibling_inline_string_containers() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cell = "<c r=\"A1\" t=\"inlineStr\"><is><t>Owen </t></is><is><t>Courser</t></is></c>";
    let path = write(
        directory.path(),
        "sibling-is.xlsx",
        &workbook(&row_with(cell)),
    )?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    Ok(())
}

#[test]
fn xlsx_profile_refuses_repeated_numeric_value_containers() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cell = "<c r=\"A1\"><v>1</v><v>2</v></c>";
    let path = write(
        directory.path(),
        "repeated-v.xlsx",
        &workbook(&row_with(cell)),
    )?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    Ok(())
}

#[test]
fn xlsx_profile_reads_rich_text_multiple_runs_in_one_inline_container() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cell = "<c r=\"A1\" t=\"inlineStr\"><is><r><t>Owen </t></r><r><t>Courser</t></r></is></c>";
    let path = write(
        directory.path(),
        "rich-text.xlsx",
        &workbook(&row_with(cell)),
    )?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = walk(&archive, "Sheet1")?;
    check!(eq; written, Some(1));
    check!(eq; walked.cells.len(), 1);
    check!(eq; walked.cells.get(&(0, 0)), Some(&Value::text("Owen Courser")));
    Ok(())
}

#[test]
fn xlsx_profile_reads_ordinary_single_numeric_value() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cell = "<c r=\"A1\"><v>42</v></c>";
    let path = write(
        directory.path(),
        "single-v.xlsx",
        &workbook(&row_with(cell)),
    )?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = walk(&archive, "Sheet1")?;
    check!(eq; written, Some(1));
    check!(eq; walked.cells.len(), 1);
    check!(eq; walked.cells.get(&(0, 0)), Some(&Value::Number(42.0)));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_declared_type_container_mismatch() -> TestResult {
    let directory = tempfile::tempdir()?;
    for cell in [
        "<c r=\"A1\"><is><t>51.12</t></is></c>",
        "<c r=\"A1\" t=\"n\"><is><t>51.12</t></is></c>",
        "<c r=\"A1\" t=\"b\"><is><t>1</t></is></c>",
        "<c r=\"A1\" t=\"str\"><is><t>51.12</t></is></c>",
        "<c r=\"A1\" t=\"d\"><is><t>2026-05-09</t></is></c>",
        "<c r=\"A1\" t=\"e\"><is><t>#VALUE!</t></is></c>",
        "<c r=\"A1\" t=\"inlineStr\"><v>51.12</v></c>",
    ] {
        let path = write(
            directory.path(),
            "wrong-container.xlsx",
            &workbook(&row_with(cell)),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let error = failure(&archive, "Sheet1")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{cell}");
    }
    Ok(())
}
