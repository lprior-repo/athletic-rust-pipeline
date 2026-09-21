use super::budget::{
    account_retained_headers, MAX_MATERIALIZED_ROW_BYTES, MAX_RETAINED_HEADER_BYTES,
};
use super::headers::{build_headers, validate_named_columns};
use super::values::materialize_cell_text;
use crate::model::{SheetStats, SourceRecord};
use anyhow::{bail, Context, Result};
use calamine::DataRef;
use std::collections::BTreeMap;

const MAX_EXCEL_ROW: u32 = 1_048_576;
const MAX_EXCEL_COLUMN: u32 = 16_384;

pub(super) struct SheetState {
    name: String,
    declared_dimension: Option<String>,
    xml_rows: u64,
    current_row: Option<u32>,
    previous_position: Option<(u32, u32)>,
    cells: BTreeMap<u32, String>,
    row_bytes: usize,
    has_data: bool,
    headers: Option<Vec<String>>,
    header_bytes: usize,
    actual_data_rows: u64,
    last_actual_row: u32,
}

impl SheetState {
    pub(super) fn new(name: &str, declared_dimension: Option<String>, xml_rows: u64) -> Self {
        Self {
            name: name.to_owned(),
            declared_dimension,
            xml_rows,
            current_row: None,
            previous_position: None,
            cells: BTreeMap::new(),
            row_bytes: 0,
            has_data: false,
            headers: None,
            header_bytes: 0,
            actual_data_rows: 0,
            last_actual_row: 0,
        }
    }

    pub(super) fn accept<F>(
        &mut self,
        cell: calamine::Cell<DataRef<'_>>,
        retained_header_bytes: &mut usize,
        callback: &mut F,
    ) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        let (row, column) = cell.get_position();
        validate_position(row, column)?;
        if self
            .previous_position
            .is_some_and(|previous| (row, column) <= previous)
        {
            bail!("worksheet cells are duplicate or out of order at ({row}, {column})");
        }
        self.previous_position = Some((row, column));
        let physical_row = row.checked_add(1).context("Excel row number overflow")?;
        if self.current_row != Some(physical_row) {
            self.finish_row(retained_header_bytes, callback)?;
            self.current_row = Some(physical_row);
            self.cells.clear();
            self.row_bytes = 0;
            self.has_data = false;
        }
        let value = materialize_cell_text(cell.get_value(), &mut self.row_bytes)?;
        if !value.is_empty() {
            self.has_data = true;
        }
        if self.cells.insert(column, value).is_some() {
            bail!("worksheet contains duplicate cell position at ({row}, {column})");
        }
        Ok(())
    }

    fn finish_row<F>(&mut self, retained_header_bytes: &mut usize, callback: &mut F) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        let Some(row) = self.current_row else {
            return Ok(());
        };
        if !self.has_data {
            return Ok(());
        }
        let cells = std::mem::take(&mut self.cells);
        if self.headers.is_none() {
            let headers = build_headers(&cells)?;
            self.header_bytes = headers
                .iter()
                .try_fold(0_usize, |total, header| total.checked_add(header.len()))
                .context("worksheet header byte count overflow")?;
            if self.header_bytes > MAX_RETAINED_HEADER_BYTES {
                bail!("worksheet headers exceed the 1 MiB retained-header limit");
            }
            account_retained_headers(retained_header_bytes, self.header_bytes)?;
            self.headers = Some(headers);
            return Ok(());
        }
        let row_bytes = self
            .row_bytes
            .checked_add(self.header_bytes)
            .context("materialized row byte count overflow")?;
        if row_bytes > MAX_MATERIALIZED_ROW_BYTES {
            bail!("materialized row keys and values exceed the 8 MiB limit");
        }
        let headers = self.headers.as_ref().context("missing worksheet headers")?;
        validate_named_columns(&cells, headers)?;
        let fields = headers
            .iter()
            .enumerate()
            .map(|(column, header)| {
                let index = u32::try_from(column).context("column index overflow")?;
                let value = cells.get(&index).map_or_else(String::new, Clone::clone);
                Ok((header.clone(), value))
            })
            .collect::<Result<BTreeMap<String, String>>>()?;
        self.actual_data_rows = self
            .actual_data_rows
            .checked_add(1)
            .context("data row count overflow")?;
        self.last_actual_row = row;
        callback(SourceRecord {
            source_key: format!("{}:{row}", self.name),
            sheet: self.name.clone(),
            excel_row: row,
            fields,
        })
    }

    pub(super) fn finish<F>(
        mut self,
        retained_header_bytes: &mut usize,
        callback: &mut F,
    ) -> Result<SheetStats>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        self.finish_row(retained_header_bytes, callback)?;
        let headers = self
            .headers
            .take()
            .context("source worksheet has no nonempty header row")?;
        Ok(SheetStats {
            name: self.name,
            declared_dimension: self.declared_dimension,
            xml_rows: self.xml_rows,
            actual_data_rows: self.actual_data_rows,
            last_actual_row: self.last_actual_row,
            headers,
        })
    }
}

fn validate_position(row: u32, column: u32) -> Result<()> {
    if row >= MAX_EXCEL_ROW || column >= MAX_EXCEL_COLUMN {
        bail!("cell position ({row}, {column}) exceeds Excel range");
    }
    Ok(())
}
