use super::fixture::shared_string_workbook;
use super::{failure, row_with, walk, workbook, write, Archive, Profile, TestResult, Value};
use crate::report::CorruptionClass;
use crate::workbook::verify::read::{Book, Budget, Readback};

const FORMULA_CELLS: [&str; 9] = [
    "<c r=\"A1\"><f>2+2</f><v>1</v></c>",
    "<c r=\"A1\"><f t=\"shared\" si=\"0\" ref=\"A1:A2\">2+2</f><v>1</v></c>",
    "<c r=\"A1\"><f t=\"shared\" si=\"0\"/><v>1</v></c>",
    "<c r=\"A1\"><f t=\"array\" ref=\"A1\">2+2</f><v>1</v></c>",
    "<c r=\"A1\"><f t=\"array\" ref=\"A1\"/><v>1</v></c>",
    "<c r=\"A1\"><f t=\"dataTable\" ref=\"A1\" r1=\"B1\"/><v>1</v></c>",
    "<c r=\"A1\"><f/><v>1</v></c>",
    "<c r=\"A1\" t=\"str\"><f>\"Changed\"</f><v>Expected</v></c>",
    "<c r=\"A1\"><f>2+2</f></c>",
];

#[test]
fn xlsx_profile_refuses_formulas_even_when_cached_values_match() -> TestResult {
    let directory = tempfile::tempdir()?;
    for cell in FORMULA_CELLS {
        let path = write(
            directory.path(),
            "cached-formula.xlsx",
            &workbook(&row_with(cell)),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let error = failure(&archive, "Sheet1")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(error.to_string().contains("formula cells"), "{error}");
        check!(error.to_string().contains("cached values"), "{error}");
    }
    Ok(())
}

#[test]
fn workbook_dispatch_cannot_bypass_formula_refusal_by_adding_shared_strings() -> TestResult {
    let directory = tempfile::tempdir()?;
    for shared in [false, true] {
        for cell in FORMULA_CELLS {
            let sheet = row_with(cell);
            let bytes = if shared {
                shared_string_workbook(&sheet)
            } else {
                workbook(&sheet)
            };
            let path = write(directory.path(), "formula-dispatch.xlsx", &bytes)?;
            let mut book = Book::open(&path)?;
            let lane = if shared {
                Readback::LegacySharedStrings
            } else {
                Readback::InlineProfile
            };
            check!(eq; book.readback(), lane);
            let mut visited = 0;
            let error = book
                .read(
                    "Sheet1",
                    Budget {
                        rows: 1,
                        columns: 1,
                    },
                    |_| visited += 1,
                )
                .err()
                .ok_or("a cached formula was accepted by the selected reader")?;
            check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
            check!(error.to_string().contains("formula cells"), "{error}");
            check!(eq; visited, 0);
        }
    }
    Ok(())
}

#[test]
fn legacy_shared_string_readback_preserves_literal_formula_prefixes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = row_with("<c r=\"A1\" t=\"s\"><v>0</v></c>");
    let path = write(
        directory.path(),
        "legacy-literal.xlsx",
        &shared_string_workbook(&sheet),
    )?;
    let mut book = Book::open(&path)?;
    check!(eq; book.readback(), Readback::LegacySharedStrings);
    let mut values = Vec::new();
    let shape = book.read(
        "Sheet1",
        Budget {
            rows: 1,
            columns: 1,
        },
        |row| {
            values.push(row.get(0).cloned());
        },
    )?;
    check!(shape.is_some());
    check!(eq; values, vec![Some(Value::text("=2+2"))]);
    Ok(())
}

#[test]
fn xlsx_profile_preserves_numbers_and_formula_prefixed_literal_text() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (cell, expected) in [
        ("<c r=\"A1\"><v>1</v></c>", Value::Number(1.0)),
        (
            "<c r=\"A1\" t=\"inlineStr\"><is><t>=2+2</t></is></c>",
            Value::text("=2+2"),
        ),
        ("<c r=\"A1\" t=\"str\"><v>=2+2</v></c>", Value::text("=2+2")),
    ] {
        let path = write(
            directory.path(),
            "literal-not-formula.xlsx",
            &workbook(&row_with(cell)),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let (walked, written) = walk(&archive, "Sheet1")?;
        check!(eq; written, Some(1));
        check!(eq; walked.cells.len(), 1);
        check!(eq; walked.cells.get(&(0, 0)), Some(&expected));
    }
    Ok(())
}
