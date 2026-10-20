use super::super::sheet_family::part_name;
use super::{Cell, SheetLayout, SheetWriter};
use crate::report::{ReportError, ReportResult};
use rust_xlsxwriter::Workbook;
use std::path::Path;
pub(super) fn write_paged<I>(
    book: &mut Workbook,
    path: &Path,
    layout: SheetLayout<'_>,
    rows: I,
    max_data_rows: usize,
) -> ReportResult<()>
where
    I: IntoIterator<Item = ReportResult<Vec<Cell>>>,
{
    if max_data_rows == 0 {
        return Err(ReportError::Invariant {
            detail: "page capacity must be greater than zero".to_string(),
        });
    }
    let mut iterator = rows.into_iter();
    let header = iterator.next().transpose()?;
    let Some(header) = header else {
        return write_empty(book, path, &layout);
    };
    let mut pending = iterator.next().transpose()?;
    let Some(first) = pending.take() else {
        return write_header_only(book, path, &layout, &header);
    };
    let mut page_index: usize = 0;
    let mut current = Some(first);
    while let Some(first_row) = current.take() {
        let next = write_data_page(
            book,
            path,
            &layout,
            (page_index, max_data_rows),
            &header,
            first_row,
            &mut iterator,
        )?;
        match next {
            None => break,
            Some(value) => {
                current = Some(value);
                page_index = page_index
                    .checked_add(1)
                    .ok_or_else(|| ReportError::Invariant {
                        detail: "page index overflow".to_string(),
                    })?;
            }
        }
    }
    Ok(())
}
fn write_empty(book: &mut Workbook, path: &Path, layout: &SheetLayout<'_>) -> ReportResult<()> {
    let name = part_name(layout.name, 0)?;
    let mut writer = SheetWriter::start(book, path, &name, layout.widths)?;
    writer.finish(0, 0, layout.chrome)
}
fn write_header_only(
    book: &mut Workbook,
    path: &Path,
    layout: &SheetLayout<'_>,
    header: &[Cell],
) -> ReportResult<()> {
    let name = part_name(layout.name, 0)?;
    let mut writer = SheetWriter::start(book, path, &name, layout.widths)?;
    writer.write_row(0, header)?;
    let last = header.len().saturating_sub(1);
    writer.finish(1, last, layout.chrome)
}
fn write_data_page<I>(
    book: &mut Workbook,
    path: &Path,
    layout: &SheetLayout<'_>,
    page: (usize, usize),
    header: &[Cell],
    first: Vec<Cell>,
    rows: &mut I,
) -> ReportResult<Option<Vec<Cell>>>
where
    I: Iterator<Item = ReportResult<Vec<Cell>>>,
{
    let (page_index, max_data_rows) = page;
    let name = part_name(layout.name, page_index)?;
    let mut writer = SheetWriter::start(book, path, &name, layout.widths)?;
    writer.write_row(0, header)?;
    let mut last_column = header.len().saturating_sub(1);
    let mut written: usize = 0;
    let mut current = Some(first);
    while written < max_data_rows {
        let Some(cells) = current.take() else {
            break;
        };
        let candidate = cells.len().saturating_sub(1);
        if candidate > last_column {
            last_column = candidate;
        }
        let index = written
            .checked_add(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "page row index overflow".to_string(),
            })?;
        writer.write_row(index, &cells)?;
        written = written
            .checked_add(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "page row count overflow".to_string(),
            })?;
        if written == max_data_rows {
            current = rows.next().transpose()?;
            break;
        }
        current = rows.next().transpose()?;
    }
    let total = written
        .checked_add(1)
        .ok_or_else(|| ReportError::Invariant {
            detail: "page total row count overflow".to_string(),
        })?;
    writer.finish(total, last_column, layout.chrome)?;
    Ok(current)
}
