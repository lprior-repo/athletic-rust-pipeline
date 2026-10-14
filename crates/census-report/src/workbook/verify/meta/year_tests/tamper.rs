use super::{fixture, options, TestResult};
use calamine::{open_workbook, Data, Reader, Xlsx};
use rust_xlsxwriter::{Workbook, Worksheet};
use std::path::Path;

#[test]
fn scalar_year_readback_refuses_2027_coverage_in_a_2028_workbook() -> TestResult {
    rejects_swapped_sheet("Coverage")
}

#[test]
fn scalar_year_readback_refuses_2027_review_in_a_2028_workbook() -> TestResult {
    rejects_swapped_sheet("Review")
}

#[test]
fn scalar_year_readback_refuses_2027_conflicts_in_a_2028_workbook() -> TestResult {
    rejects_swapped_sheet("Conflicts")
}

fn rejects_swapped_sheet(name: &str) -> TestResult {
    let fixture = fixture()?;
    let (legacy, _) = fixture.publish(Some(2027))?;
    let (selected, dataset) = fixture.publish(Some(2028))?;
    crate::workbook::verify::verify_frozen(&selected, &dataset, &options(Some(2028))?)?;
    let target = fixture
        .directory
        .path()
        .join(format!("swapped-{name}.xlsx"));
    swap_sheet(&selected, &legacy, &target, name)?;
    let error = crate::workbook::verify::verify_frozen(&target, &dataset, &options(Some(2028))?)
        .err()
        .ok_or_else(|| format!("2027 {name} accepted in a 2028 workbook"))?;
    check!(error.to_string().contains(name));
    Ok(())
}

fn swap_sheet(source: &Path, replacement: &Path, target: &Path, name: &str) -> TestResult {
    let mut original: Xlsx<_> = open_workbook(source)?;
    let mut legacy: Xlsx<_> = open_workbook(replacement)?;
    let replacement = legacy.worksheet_range(name)?;
    let mut writer = Workbook::new();
    let mut changed = 0_usize;
    for sheet_name in original.sheet_names() {
        let original_range = original.worksheet_range(&sheet_name)?;
        let range = if sheet_name == name {
            changed = changed.checked_add(1).ok_or("swap count overflow")?;
            &replacement
        } else {
            &original_range
        };
        let sheet = writer.add_worksheet();
        sheet.set_name(&sheet_name)?;
        for (row, cells) in range.rows().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                write_cell(sheet, u32::try_from(row)?, u16::try_from(column)?, cell)?;
            }
        }
    }
    check!(eq; changed, 1);
    writer.save(target)?;
    Ok(())
}

fn write_cell(sheet: &mut Worksheet, row: u32, column: u16, value: &Data) -> TestResult {
    match value {
        Data::Empty => {}
        Data::Float(value) => {
            sheet.write_number(row, column, *value)?;
        }
        Data::Int(value) => {
            sheet.write_number(row, column, value.to_string().parse::<f64>()?)?;
        }
        _ => {
            sheet.write_string(row, column, value.to_string())?;
        }
    }
    Ok(())
}
