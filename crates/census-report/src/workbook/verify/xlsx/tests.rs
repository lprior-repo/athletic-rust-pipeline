mod container;
mod deflate;
mod fixture;
mod formulas;
mod names;
mod parity;
mod publication;
mod relationships;

use crate::report::CorruptionClass;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::read::tests::TestResult;
use crate::workbook::verify::xlsx::{Archive, Profile};
use fixture::{
    archive, byte_profile, failure, inline_sheet, row_with, sheet_with, token_profile, walk,
    workbook, write,
};
use std::collections::BTreeMap;

#[test]
fn xlsx_profile_reads_every_inline_cell_without_sampling() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = write(directory.path(), "inline.xlsx", &workbook(&inline_sheet()))?;
    let archive = Archive::open(&path, Profile::inline())?;
    check!(eq; archive.sheet_names()?, vec!["Sheet1".to_string()]);
    check!(
        !archive.uses_shared_strings(),
        "the fixture stores inline strings"
    );
    let (walked, written) = walk(&archive, "Sheet1")?;
    let expected: BTreeMap<(usize, usize), Value> = [
        ((0, 0), Value::text("Name")),
        ((0, 1), Value::text("Mark")),
        ((0, 2), Value::text("Team & School")),
        ((1, 0), Value::text(" Oracle  Runner ")),
        ((1, 1), Value::Number(11.5)),
        ((1, 2), Value::text("true")),
    ]
    .into_iter()
    .collect();
    check!(eq; walked.cells, expected, "cell by cell readback");
    check!(eq; walked.rows, Some(3), "the declared dimension spans three rows");
    check!(eq; written, Some(3));
    Ok(())
}

#[test]
fn xlsx_profile_reads_self_closing_elements_without_losing_nesting() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = sheet_with(
        "A1:B1",
        "<row r=\"1\"><c r=\"A1\"/><c r=\"B1\"><v>2</v></c></row>",
    );
    let path = write(directory.path(), "self-closing.xlsx", &workbook(&sheet))?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = walk(&archive, "Sheet1")?;
    let expected: BTreeMap<(usize, usize), Value> =
        [((0, 0), Value::Empty), ((0, 1), Value::Number(2.0))]
            .into_iter()
            .collect();
    check!(eq; walked.cells, expected, "a self-closing cell is empty and its row still closes");
    check!(eq; walked.rows, Some(1), "the declared dimension spans one row");
    check!(eq; written, Some(1));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_shared_string_sheets_and_unknown_types() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cases = [
        (
            "shared",
            row_with("<c r=\"A1\" t=\"s\"><v>0</v></c>"),
            "sharedStrings.xml",
        ),
        (
            "unknown",
            row_with("<c r=\"A1\" t=\"zz\"><v>0</v></c>"),
            "not part of the profile",
        ),
    ];
    for (label, sheet, expected) in cases {
        let path = write(
            directory.path(),
            &format!("{label}.xlsx"),
            &workbook(&sheet),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let error = failure(&archive, "Sheet1")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{label}: {error}");
        check!(error.to_string().contains(expected), "{label}: {error}");
    }
    Ok(())
}

#[test]
fn xlsx_profile_refuses_a_shared_string_target_rather_than_reading_blanks() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = row_with("<c r=\"A1\" t=\"s\"><v>3</v></c>");
    let path = write(directory.path(), "shared-refused.xlsx", &workbook(&sheet))?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = failure(&archive, "Sheet1")?;
    check!(error.to_string().contains("refused"), "{error}");
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_xml_tokens_beyond_the_budget() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cases = [(32_usize, 8_usize, "token"), (200, 256, "text run")];
    for (budget, run, expected) in cases {
        let text = "a".repeat(run);
        let sheet = row_with(&format!(
            "<c r=\"A1\" t=\"inlineStr\"><is><t>{text}</t></is></c>"
        ));
        let path = write(
            directory.path(),
            &format!("token-{budget}.xlsx"),
            &workbook(&sheet),
        )?;
        let archive = Archive::open(&path, token_profile(budget))?;
        let error = failure(&archive, "Sheet1")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{budget}: {error}");
        check!(
            error.to_string().contains("byte budget"),
            "{budget}: {error}"
        );
        check!(error.to_string().contains(expected), "{budget}: {error}");
    }
    Ok(())
}

#[test]
fn verify_corrupt_xml_sheets_are_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cases = [
        (
            "open-elements",
            "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData><row r=\"1\"><c r=\"A1\"><v>1</v></c></row>".to_string(),
            "still open",
        ),
        (
            "malformed-reference",
            sheet_with("A1:B2", "<row r=\"1\"><c r=\"A0\"><v>1</v></c></row>"),
            "A0 is malformed",
        ),
        (
            "beyond-dimension",
            sheet_with("A1:B2", "<row r=\"3\"><c r=\"C3\"><v>1</v></c></row>"),
            "beyond its declared dimension",
        ),
        (
            "out-of-order",
            sheet_with(
                "A1:B2",
                "<row r=\"1\"><c r=\"B1\"><v>1</v></c><c r=\"A1\"><v>2</v></c></row>",
            ),
            "revisits cell A1",
        ),
        (
            "not-a-number",
            sheet_with("A1:B2", "<row r=\"1\"><c r=\"A1\"><v>abc</v></c></row>"),
            "not a number",
        ),
        (
            "nesting",
            sheet_with("A1:B2", "<row r=\"1\"></sheetData></row>"),
            "inconsistently",
        ),
        (
            "unknown-entity",
            sheet_with(
                "A1:B2",
                "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>&fake;</t></is></c></row>",
            ),
            "not recognised",
        ),
        (
            "cdata",
            sheet_with(
                "A1:B2",
                "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t><![CDATA[x]]></t></is></c></row>",
            ),
            "CDATA",
        ),
    ];
    for (label, sheet, expected) in cases {
        let path = write(
            directory.path(),
            &format!("{label}.xlsx"),
            &workbook(&sheet),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let error = failure(&archive, "Sheet1")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{label}: {error}");
        check!(error.to_string().contains(expected), "{label}: {error}");
    }
    Ok(())
}

#[test]
fn verify_corrupt_declared_members_beyond_the_byte_budget_are_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = row_with("<c r=\"A1\"><v>1</v></c>");
    let bytes = archive(&[("xl/worksheets/sheet1.xml", sheet.as_bytes(), 0, 1_000_000)]);
    let path = write(directory.path(), "declared.xlsx", &bytes)?;
    let error = Archive::open(&path, byte_profile(1 << 12))
        .err()
        .ok_or("an oversized declaration was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("readback budget"), "{error}");
    Ok(())
}

#[test]
fn verify_corrupt_oversize_decompressed_stream_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut body = String::new();
    for row in 0..512 {
        body.push_str(&format!(
            "<row r=\"{row}\"><c r=\"A{row}\" t=\"inlineStr\"><is><t>padding</t></is></c></row>"
        ));
    }
    let sheet = sheet_with("A1:A512", &body);
    check!(
        sheet.len() > 4_096,
        "the fixture must inflate beyond the budget"
    );
    let stream = fixture::deflate_stored(sheet.as_bytes());
    let bytes = fixture::workbook_archive(&stream, 8, 1_024);
    let path = write(directory.path(), "inflated.xlsx", &bytes)?;
    let archive = Archive::open(&path, byte_profile(4_096))?;
    let error = failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("4096"), "{error}");
    check!(error.to_string().contains("readback budget"), "{error}");
    Ok(())
}

#[test]
fn verify_corrupt_container_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let bytes = workbook(&inline_sheet());
    check!(bytes.len() > 64, "the fixture container is trivially short");
    let cut = bytes.len().saturating_sub(30);
    let path = write(directory.path(), "truncated.xlsx", &bytes[..cut])?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("a truncated container was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("end record"), "{error}");
    Ok(())
}

#[test]
fn verify_corrupt_archive_without_a_worksheet_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let bytes = archive(&[("[Content_Types].xml", b"<Types/>", 0, 8)]);
    let path = write(directory.path(), "nameless.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = archive
        .sheet_names()
        .err()
        .ok_or("an archive without a workbook was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("xl/workbook.xml"), "{error}");
    Ok(())
}
