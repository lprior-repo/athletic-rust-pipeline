use calamine::{Cell, DataRef, XlsxCellReader};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::{unreadable, visit_row, Budget, ReportError, ReportResult, Shape, SparseRow, Value};

struct ReadScope<'a> {
    path: &'a Path,
    name: &'a str,
    budget: Budget,
}

pub(super) fn read(
    reader: &mut XlsxCellReader<'_, BufReader<File>>,
    path: &Path,
    name: &str,
    budget: Budget,
    visit: &mut impl FnMut(&SparseRow),
) -> ReportResult<Shape> {
    let scope = ReadScope { path, name, budget };
    let rows = declared_shape(reader, &scope)?;
    let mut pending = PendingRows::default();
    while let Some(cell) = reader
        .next_cell()
        .map_err(|source| unreadable(scope.path, source))?
    {
        pending.append(cell, &scope, visit)?;
    }
    pending.finish(rows, visit);
    Ok(Shape { rows })
}

fn declared_shape(
    reader: &XlsxCellReader<'_, BufReader<File>>,
    scope: &ReadScope<'_>,
) -> ReportResult<usize> {
    let dimensions = reader.dimensions();
    let rows = extent(dimensions.end.0);
    let columns = extent(dimensions.end.1);
    scope.budget.validate(scope.name, rows, columns)?;
    Ok(rows)
}

fn extent(position: u32) -> usize {
    usize::try_from(position)
        .map_or(usize::MAX, |value| value)
        .saturating_add(1)
}

#[derive(Default)]
struct PendingRows {
    held: Option<SparseRow>,
    next: usize,
}

impl PendingRows {
    fn append(
        &mut self,
        cell: Cell<DataRef<'_>>,
        scope: &ReadScope<'_>,
        visit: &mut impl FnMut(&SparseRow),
    ) -> ReportResult<()> {
        let (row, column) = position(&cell, scope)?;
        if self
            .held
            .as_ref()
            .is_none_or(|current| current.index != row)
        {
            if let Some(finished) = self.held.take() {
                self.next = visit_row(&finished, self.next, visit);
            }
            self.held = Some(SparseRow::new(row));
        }
        if let Some(current) = self.held.as_mut() {
            current
                .cells
                .insert(column, Value::from_ref(cell.get_value()));
        }
        Ok(())
    }

    fn finish(&mut self, rows: usize, visit: &mut impl FnMut(&SparseRow)) {
        if let Some(finished) = self.held.take() {
            self.next = visit_row(&finished, self.next, visit);
        }
        (self.next..rows).for_each(|index| visit(&SparseRow::new(index)));
    }
}

fn position(cell: &Cell<DataRef<'_>>, scope: &ReadScope<'_>) -> ReportResult<(usize, usize)> {
    let row = usize::try_from(cell.get_position().0).map_or(usize::MAX, |value| value);
    let column = usize::try_from(cell.get_position().1).map_or(usize::MAX, |value| value);
    if row >= scope.budget.rows || column >= scope.budget.columns {
        return Err(ReportError::Invariant {
            detail: format!(
                "sheet {} contains an out-of-budget cell at {row},{column}",
                scope.name
            ),
        });
    }
    Ok((row, column))
}
