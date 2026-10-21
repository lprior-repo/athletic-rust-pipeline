use super::super::tests::TestResult;
use super::super::{Book, Budget, Readback, Shape};
use crate::report::{ReportError, ReportResult};
use crate::workbook::verify::canonical::Value;
use calamine::{Data, Range, Reader};
use rust_xlsxwriter::Workbook;
use std::path::Path;

type Cells = Vec<(usize, Value, Value)>;
type Page<'a> = (&'a str, &'a [(&'a str, f64)]);

pub(super) fn workbook(path: &Path, pages: &[Page<'_>], lane: Readback) -> TestResult {
    let mut workbook = Workbook::new();
    for (name, rows) in pages {
        let sheet = match lane {
            Readback::InlineProfile => workbook.add_worksheet_with_constant_memory()?,
            Readback::LegacySharedStrings => workbook.add_worksheet(),
        };
        sheet.set_name(*name)?;
        sheet.write_string(0, 0, "Name")?;
        sheet.write_string(0, 1, "Mark")?;
        for (index, (name, mark)) in rows.iter().enumerate() {
            let row = u32::try_from(index)?.checked_add(1).ok_or("row overflow")?;
            sheet.write_string(row, 0, *name)?;
            sheet.write_number(row, 1, *mark)?;
        }
    }
    workbook.save(path)?;
    Ok(())
}

pub(super) fn collect(book: &mut Book, budget: Budget) -> TestResult<(Option<usize>, Cells)> {
    let mut cells = Vec::new();
    let shape = book.read_family("Review", budget, |row| {
        cells.push((
            row.index(),
            row.get(0).cloned().map_or(Value::Empty, |value| value),
            row.get(1).cloned().map_or(Value::Empty, |value| value),
        ));
    })?;
    Ok((shape.map(|shape| shape.rows), cells))
}

pub(super) fn refused(result: ReportResult<Option<Shape>>, path: &Path) -> TestResult {
    match result {
        Err(ReportError::ArtifactCorrupt { path: found, .. }) => check!(eq; found, path),
        Err(error) => return Err(format!("unexpected refusal: {error}").into()),
        Ok(_) => return Err("malformed family accepted".into()),
    }
    Ok(())
}

pub(super) fn partition(path: &Path, base: &str, capacity: usize) -> TestResult<usize> {
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let names = source.sheet_names().to_vec();
    let mut workbook = Workbook::new();
    let mut pages = 0_usize;
    for name in names {
        let range = source.worksheet_range(&name)?;
        if name != base {
            write_range(&mut workbook, &name, &range)?;
            continue;
        }
        let rows: Vec<_> = range.rows().collect();
        let header = rows.first().ok_or("missing header")?;
        let data = rows.get(1..).ok_or("missing data")?;
        for (page, chunk) in data.chunks(capacity).enumerate() {
            let ordinal = page.checked_add(1).ok_or("page overflow")?;
            let title = if page == 0 {
                base.to_owned()
            } else {
                format!("{base}_{ordinal:03}")
            };
            let last_column = header.len().checked_sub(1).ok_or("empty header")?;
            let mut cells = Range::new(
                (0, 0),
                (u32::try_from(chunk.len())?, u32::try_from(last_column)?),
            );
            for (row, values) in std::iter::once(*header)
                .chain(chunk.iter().copied())
                .enumerate()
            {
                for (column, value) in values.iter().enumerate() {
                    cells.set_value((u32::try_from(row)?, u32::try_from(column)?), value.clone());
                }
            }
            write_range(&mut workbook, &title, &cells)?;
            pages = pages.checked_add(1).ok_or("page count overflow")?;
        }
    }
    check!(pages >= 2, "fixture must physically span multiple pages");
    workbook.save(path)?;
    Ok(pages)
}

pub(super) fn rename(path: &Path, old: &str, new: &str) -> TestResult {
    rewrite(path, |name, range, workbook| {
        write_range(workbook, if name == old { new } else { name }, range)
    })
}

pub(super) fn extra_page(path: &Path, name: &str, header_only: bool) -> TestResult {
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let names = source.sheet_names().to_vec();
    let mut workbook = Workbook::new();
    for name in names {
        write_range(&mut workbook, &name, &source.worksheet_range(&name)?)?;
    }
    let mut range = source.worksheet_range("Coaches_002")?;
    if header_only {
        let header = range.rows().next().ok_or("extra page header")?;
        let last_column = header.len().checked_sub(1).ok_or("empty header")?;
        let mut cells = Range::new((0, 0), (0, u32::try_from(last_column)?));
        for (column, value) in header.iter().enumerate() {
            cells.set_value((0, u32::try_from(column)?), value.clone());
        }
        range = cells;
    }
    write_range(&mut workbook, name, &range)?;
    workbook.save(path)?;
    Ok(())
}

fn rewrite(
    path: &Path,
    mut write: impl FnMut(&str, &Range<Data>, &mut Workbook) -> TestResult,
) -> TestResult {
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let names = source.sheet_names().to_vec();
    let mut workbook = Workbook::new();
    for name in names {
        write(&name, &source.worksheet_range(&name)?, &mut workbook)?;
    }
    workbook.save(path)?;
    Ok(())
}

fn write_range(workbook: &mut Workbook, name: &str, range: &Range<Data>) -> TestResult {
    let sheet = workbook.add_worksheet_with_constant_memory()?;
    sheet.set_name(name)?;
    let (first_row, first_column) = range.start().map_or((0, 0), |position| position);
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
            other => return Err(format!("unsupported fixture cell {other:?}").into()),
        }
    }
    if let Some((row, _)) = range.end() {
        sheet.set_row_height(row, 15)?;
    }
    Ok(())
}
