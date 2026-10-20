use super::{Book, Budget, Shape, SparseRow, Value, EXCEL_COLUMNS_PER_SHEET, EXCEL_ROWS_PER_SHEET};
use crate::report::{artifact_corrupt, verifier_input, ReportResult};
use crate::workbook::sheet_family::{part_index, part_name};
use std::collections::BTreeMap;
use std::path::Path;

impl Budget {
    pub(in crate::workbook::verify) fn family(data_rows: usize) -> ReportResult<Self> {
        let rows = data_rows
            .checked_add(1)
            .ok_or_else(|| verifier_input("the logical worksheet row budget overflows"))?;
        Ok(Self {
            rows,
            columns: EXCEL_COLUMNS_PER_SHEET,
        })
    }

    fn physical(self) -> ReportResult<Self> {
        let physical = Self {
            rows: self.rows.min(EXCEL_ROWS_PER_SHEET),
            columns: self.columns,
        };
        physical.usable()?;
        Ok(physical)
    }
}

impl Book {
    pub(in crate::workbook::verify) fn read_family(
        &mut self,
        base: &str,
        budget: Budget,
        mut visit: impl FnMut(&SparseRow),
    ) -> ReportResult<Option<Shape>> {
        let physical = budget.physical()?;
        let names = names(&self.path, &self.names, base)?;
        if names.is_empty() {
            return Ok(None);
        }
        let path = self.path.clone();
        let mut family = Family {
            path: &path,
            base,
            budget,
            header: None,
            rows: 0,
        };
        names.iter().enumerate().try_for_each(|(page, name)| {
            family.read_page(self, name, page, physical, &mut visit)
        })?;
        Ok(Some(Shape { rows: family.rows }))
    }
}

pub(super) fn names(path: &Path, sheets: &[String], base: &str) -> ReportResult<Vec<String>> {
    let prefix = format!("{base}_");
    let mut pages = BTreeMap::new();
    sheets
        .iter()
        .filter(|name| name.as_str() == base || name.starts_with(&prefix))
        .try_for_each(|name| {
            let page = part_index(base, name).ok_or_else(|| {
                artifact_corrupt(
                    path,
                    format!("sheet family {base} has noncanonical page {name}"),
                )
            })?;
            if pages.insert(page, name).is_some() {
                return Err(artifact_corrupt(
                    path,
                    format!("sheet family {base} repeats page {name}"),
                ));
            }
            Ok(())
        })?;
    pages
        .into_iter()
        .enumerate()
        .map(|(expected, (page, name))| {
            if page != expected {
                return Err(artifact_corrupt(
                    path,
                    format!(
                        "sheet family {base} is missing {} before {name}",
                        part_name(base, expected)?
                    ),
                ));
            }
            Ok(name.clone())
        })
        .collect()
}

struct Family<'a> {
    path: &'a Path,
    base: &'a str,
    budget: Budget,
    header: Option<BTreeMap<usize, Value>>,
    rows: usize,
}

impl Family<'_> {
    fn read_page(
        &mut self,
        book: &mut Book,
        name: &str,
        page: usize,
        physical: Budget,
        visit: &mut impl FnMut(&SparseRow),
    ) -> ReportResult<()> {
        let mut error = None;
        let offset = self.rows;
        let mut next = offset;
        let shape = book.read_rows(name, physical, |row| {
            if error.is_none() {
                if let Err(failure) = self.visit(row, offset, page, &mut next, visit) {
                    error = Some(failure);
                }
            }
        })?;
        if let Some(error) = error {
            return Err(error);
        }
        let shape = shape.ok_or_else(|| self.corrupt(format!("page {name} disappeared")))?;
        if shape.rows == 0 || (page > 0 && shape.rows < 2) {
            return Err(self.corrupt(format!("page {name} has no header or continuation data")));
        }
        self.rows = next;
        Ok(())
    }

    fn visit(
        &mut self,
        row: &mut SparseRow,
        offset: usize,
        page: usize,
        next: &mut usize,
        visit: &mut impl FnMut(&SparseRow),
    ) -> ReportResult<()> {
        if row.index == 0 {
            if page > 0 {
                if self.header.as_ref() != Some(&row.cells) {
                    return Err(self.corrupt(format!("page {page} changes the complete header")));
                }
                return Ok(());
            }
            self.header = Some(row.cells.clone());
        }
        let local = row
            .index
            .checked_sub(usize::from(page > 0))
            .ok_or_else(|| self.corrupt("a page row precedes its data"))?;
        let logical = offset
            .checked_add(local)
            .ok_or_else(|| self.corrupt("the logical row index overflows"))?;
        if logical >= self.budget.rows {
            return Err(self.corrupt("the logical row count exceeds the readback budget"));
        }
        *next = logical
            .checked_add(1)
            .ok_or_else(|| self.corrupt("the logical row count overflows"))?;
        let physical = row.index;
        row.index = logical;
        visit(row);
        row.index = physical;
        Ok(())
    }

    fn corrupt(&self, detail: impl AsRef<str>) -> crate::report::ReportError {
        artifact_corrupt(
            self.path,
            format!("sheet family {}: {}", self.base, detail.as_ref()),
        )
    }
}
