use super::*;
use crate::report::ReportError;
use rust_xlsxwriter::{Workbook, Worksheet};

mod manifest;

fn copy_cell(sheet: &mut Worksheet, row: u32, column: u16, value: &Data) -> TestResult {
    match value {
        Data::Empty => (),
        Data::String(value) => {
            sheet.write_string(row, column, value)?;
        }
        Data::Float(value) => {
            sheet.write_number(row, column, *value)?;
        }
        Data::Int(value) => {
            sheet.write_number(row, column, f64::from(i32::try_from(*value)?))?;
        }
        Data::Bool(value) => {
            sheet.write_boolean(row, column, *value)?;
        }
        other => return Err(format!("unsupported fixture workbook cell {other:?}").into()),
    }
    Ok(())
}

fn copied_workbook(path: &Path, state: &str) -> TestResult<Vec<u8>> {
    let mut original: Xlsx<_> = open_workbook(path)?;
    let names = original.sheet_names().to_vec();
    let mut copied = Workbook::new();
    let mut changed = 0_usize;
    names.iter().try_for_each(|name| -> TestResult {
        let range = original.worksheet_range(name)?;
        let sheet = copied.add_worksheet();
        sheet.set_name(name)?;
        check!(eq; range.start(), Some((0, 0)), "{name}");
        range
            .rows()
            .enumerate()
            .try_for_each(|(row_index, row)| -> TestResult {
                row.iter()
                    .enumerate()
                    .try_for_each(|(column_index, value)| -> TestResult {
                        let row = u32::try_from(row_index)?;
                        let column = u16::try_from(column_index)?;
                        if name == "PRs" && row_index == 1 && column_index == 4 {
                            check!(eq; value, &Data::String("UNKNOWN".into()));
                            sheet.write_string(row, column, state)?;
                            changed = changed
                                .checked_add(1)
                                .ok_or("fixture cell count exhausted")?;
                            return Ok(());
                        }
                        copy_cell(sheet, row, column, value)
                    })
            })
    })?;
    check!(eq; changed, 1);
    Ok(copied.save_to_buffer()?)
}

fn assert_rebound_cell(path: &Path, expected: &str) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let prs = book.worksheet_range("PRs")?;
    check!(eq; cell(&prs, "State")?, &Data::String(expected.into()));
    check!(eq; cell(&prs, "School")?, &Data::String(HISTORICAL_SCHOOL.into()));
    check!(eq; cell(&book.worksheet_range("Athletes")?, "State")?, &Data::String("IL".into()));
    Ok(())
}

fn assert_semantic_refusal(path: &Path) -> TestResult {
    let detail = match crate::workbook::publication::verify_published(path) {
        Err(ReportError::SemanticMismatch {
            path: found,
            detail,
        }) => {
            check!(eq; found.as_path(), path);
            detail
        }
        Err(error) => {
            return Err(format!("unexpected historical-affiliation refusal: {error}").into())
        }
        Ok(_) => {
            return Err("current Illinois state replaced historical UNKNOWN without refusal".into())
        }
    };
    check!(
        detail.contains("frozen workbook readback found 1 defect(s)"),
        "{detail}"
    );
    check!(detail.contains("PRs!E2"), "{detail}");
    check!(
        detail.contains("expected \"UNKNOWN\", found \"IL\""),
        "{detail}"
    );
    check!(!detail.contains("generation artifact mismatch"), "{detail}");
    check!(!detail.contains("corrupt generation manifest"), "{detail}");
    Ok(())
}

#[test]
fn independent_readback_refuses_current_state_relabeling_after_workbook_inventory_rebind(
) -> TestResult {
    let fixture = Fixture::new(AffiliationCase::UnknownHistoricalState)?;
    let directory = tempfile::tempdir()?;
    let path = publish(directory.path(), &fixture)?;
    let original = path
        .parent()
        .ok_or("missing original historical-affiliation generation")?;
    let workbook_bytes = std::fs::read(&path)?;
    let frozen_bytes = std::fs::read(original.join("frozen-input.json"))?;
    assert_rebound_cell(&path, "UNKNOWN")?;
    let unchanged = manifest::install(
        original,
        &directory.path().join("unchanged"),
        "workbook.xlsx",
        &workbook_bytes,
    )?;
    check!(eq; std::fs::read(&unchanged)?, workbook_bytes);
    check!(eq; crate::workbook::publication::verify_published(&unchanged)?.workbook.mapped_athletes, 1);
    let copied = manifest::install(
        original,
        &directory.path().join("copied"),
        "workbook.xlsx",
        &copied_workbook(&path, "UNKNOWN")?,
    )?;
    check!(eq; crate::workbook::publication::verify_published(&copied)?.workbook.mapped_athletes, 1);
    let forged = manifest::install(
        original,
        &directory.path().join("forged"),
        "workbook.xlsx",
        &copied_workbook(&path, "IL")?,
    )?;
    assert_rebound_cell(&forged, "IL")?;
    check!(eq; std::fs::read(forged.parent().ok_or("forged generation")?.join("frozen-input.json"))?, frozen_bytes);
    assert_semantic_refusal(&forged)?;
    check!(eq; std::fs::read(&path)?, workbook_bytes);
    check!(eq; std::fs::read(original.join("frozen-input.json"))?, frozen_bytes);
    check!(eq; crate::workbook::publication::verify_published(&path)?.workbook.mapped_athletes, 1);
    Ok(())
}
