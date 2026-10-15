use super::TestResult;
use crate::export::ExportDataset;
use crate::workbook::Options;
use calamine::{Data, Range, Reader};
use census_domain::model::GradYear;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

pub(in crate::workbook::verify) fn copy_bundle(path: &Path, target: &Path) -> TestResult<PathBuf> {
    let generation = path.parent().ok_or("generation")?;
    let name = generation.file_name().ok_or("generation name")?;
    let copy = target.join("generations").join(name);
    std::fs::create_dir_all(&copy)?;
    for entry in std::fs::read_dir(generation)? {
        let entry = entry?;
        std::fs::copy(entry.path(), copy.join(entry.file_name()))?;
    }
    Ok(copy.join("workbook.xlsx"))
}

pub(in crate::workbook::verify) fn alter_sheet(
    path: &Path,
    sheet: &str,
    alter: impl FnOnce(&mut Range<Data>) -> TestResult,
) -> TestResult {
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let names = source.sheet_names().to_vec();
    let mut workbook = Workbook::new();
    let mut alter = Some(alter);
    for name in names {
        let mut range = source.worksheet_range(&name)?;
        if name == sheet {
            alter.take().ok_or("duplicate target worksheet")?(&mut range)?;
        }
        write_sheet(&mut workbook, &name, &range)?;
    }
    check!(alter.is_none(), "missing worksheet {sheet}");
    workbook.save(path)?;
    Ok(())
}

fn write_sheet(workbook: &mut Workbook, name: &str, range: &Range<Data>) -> TestResult {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;
    let (first_row, first_column) = range.start().map_or((0, 0), |value| value);
    for (row, column, value) in range.used_cells() {
        let row = first_row
            .checked_add(u32::try_from(row)?)
            .ok_or("row overflow")?;
        let column = first_column
            .checked_add(u32::try_from(column)?)
            .ok_or("column overflow")?;
        let column = u16::try_from(column)?;
        match value {
            Data::String(value) => {
                worksheet.write_string(row, column, value)?;
            }
            Data::Float(value) => {
                worksheet.write_number(row, column, *value)?;
            }
            Data::Int(value) => {
                worksheet.write_number(row, column, f64::from(i32::try_from(*value)?))?;
            }
            Data::Bool(value) => {
                worksheet.write_boolean(row, column, *value)?;
            }
            other => return Err(format!("unsupported fixture cell {other:?}").into()),
        }
    }
    if let Some((last_row, _)) = range.end() {
        worksheet.set_row_height(last_row, 15)?;
    }
    Ok(())
}

pub(in crate::workbook::verify) fn remove_row(range: &mut Range<Data>, row: u32) -> TestResult {
    let (_, last_column) = range.end().ok_or("empty fixture range")?;
    for column in 0..=last_column {
        range.set_value((row, column), Data::Empty);
    }
    Ok(())
}

pub(in crate::workbook::verify) fn replace_cell(
    range: &mut Range<Data>,
    cell: (u32, u32),
    text: &str,
) {
    range.set_value(cell, Data::String(text.to_string()));
}

pub(in crate::workbook::verify) fn recapture(
    path: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> TestResult {
    let directory = path.parent().ok_or("generation")?;
    crate::workbook::publication::recapture_manifest(directory, dataset, options)?;
    Ok(())
}

pub(in crate::workbook::verify) fn accepted_by_all(
    path: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> TestResult {
    use crate::workbook::{publication, verify::verify_frozen};
    for workbook in [
        verify_frozen(path, dataset, options)?,
        publication::verify_published(path)?.workbook,
        publication::open_publication(path, &dataset.lineage, options.scope, GradYear::CO2027)?
            .workbook,
    ] {
        check!(eq; workbook.mapped_athletes, 1);
    }
    Ok(())
}

pub(in crate::workbook::verify) fn rejected_by_all(
    path: &Path,
    dataset: &ExportDataset,
    options: &Options,
    location: &str,
) -> TestResult {
    use crate::workbook::{publication, verify::verify_frozen};
    for result in [
        verify_frozen(path, dataset, options).map(|_| ()),
        publication::verify_published(path).map(|_| ()),
        publication::open_publication(path, &dataset.lineage, options.scope, GradYear::CO2027)
            .map(|_| ()),
    ] {
        match result {
            Err(crate::report::ReportError::SemanticMismatch {
                path: found,
                detail,
            }) => {
                check!(eq; found, path);
                check!(detail.contains(location), "{location}: {detail}");
            }
            Err(error) => return Err(format!("{location}: unexpected refusal: {error}").into()),
            Ok(()) => return Err(format!("{location}: forged workbook accepted").into()),
        }
    }
    Ok(())
}
