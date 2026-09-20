use crate::model::{SheetStats, SourceRecord};
use anyhow::{bail, Context, Result};
use calamine::{DataRef, Xlsx};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Seek},
};

const MAX_EXCEL_ROW: u32 = 1_048_576;
const MAX_EXCEL_COLUMN: u32 = 16_384;
pub(crate) const MAX_MATERIALIZED_ROW_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAX_RETAINED_HEADER_BYTES: usize = 1024 * 1024;

pub(crate) fn visit_sheet<RS, F>(
    workbook: &mut Xlsx<RS>,
    name: &str,
    declared_dimension: Option<String>,
    xml_rows: u64,
    retained_header_bytes: &mut usize,
    callback: &mut F,
) -> Result<SheetStats>
where
    RS: Read + Seek,
    F: FnMut(SourceRecord) -> Result<()>,
{
    let mut reader = workbook.worksheet_cells_reader(name)?;
    let mut state = SheetState::new(name, declared_dimension, xml_rows);
    let mut done = false;
    std::iter::from_fn(|| {
        if done {
            return None;
        }
        match reader.next_cell() {
            Ok(Some(cell)) => Some(Ok(cell)),
            Ok(None) => {
                done = true;
                None
            }
            Err(error) => {
                done = true;
                Some(Err(error))
            }
        }
    })
    .try_for_each(|cell| state.accept(cell?, retained_header_bytes, callback))?;
    state.finish(retained_header_bytes, callback)
}

struct SheetState {
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
    fn new(name: &str, declared_dimension: Option<String>, xml_rows: u64) -> Self {
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

    fn accept<F>(
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

    fn finish<F>(
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

fn validate_named_columns(cells: &BTreeMap<u32, String>, headers: &[String]) -> Result<()> {
    cells.iter().try_for_each(|(column, value)| {
        let index = usize::try_from(*column).context("worksheet column conversion overflow")?;
        if !value.is_empty() && index >= headers.len() {
            bail!("worksheet data appears in unnamed column {column}");
        }
        Ok(())
    })
}

fn build_headers(cells: &BTreeMap<u32, String>) -> Result<Vec<String>> {
    let last = cells
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(column, _)| *column)
        .max()
        .context("source worksheet has no nonempty headers")?;
    let width = usize::try_from(last)
        .ok()
        .and_then(|value| value.checked_add(1))
        .context("worksheet header width overflow")?;
    let headers: Vec<String> = (0..width)
        .map(|column| {
            u32::try_from(column)
                .ok()
                .and_then(|index| cells.get(&index).cloned())
                .unwrap_or_default()
        })
        .collect();
    if headers.iter().any(String::is_empty) {
        bail!("source worksheet contains an empty data header");
    }
    let mut seen = BTreeSet::new();
    headers.iter().try_for_each(|header| {
        if !seen.insert(header) {
            bail!("source worksheet contains repeated data header {header:?}");
        }
        Ok(())
    })?;
    Ok(headers)
}

fn validate_position(row: u32, column: u32) -> Result<()> {
    if row >= MAX_EXCEL_ROW || column >= MAX_EXCEL_COLUMN {
        bail!("cell position ({row}, {column}) exceeds Excel range");
    }
    Ok(())
}
fn materialize_cell_text(value: &DataRef<'_>, row_bytes: &mut usize) -> Result<String> {
    match value {
        DataRef::String(text) => {
            account_row_bytes(row_bytes, text.len())?;
            Ok(text.clone())
        }
        DataRef::SharedString(text) => {
            account_row_bytes(row_bytes, text.len())?;
            Ok((*text).to_owned())
        }
        DataRef::DateTimeIso(text) | DataRef::DurationIso(text) => {
            account_row_bytes(row_bytes, text.len())?;
            Ok(text.clone())
        }
        other => {
            let text = cell_text(other)?;
            account_row_bytes(row_bytes, text.len())?;
            Ok(text)
        }
    }
}

pub(crate) fn account_retained_headers(retained: &mut usize, added: usize) -> Result<()> {
    *retained = retained
        .checked_add(added)
        .context("retained workbook header byte count overflow")?;
    if *retained > MAX_RETAINED_HEADER_BYTES {
        bail!("retained workbook headers exceed the 1 MiB limit");
    }
    Ok(())
}
fn account_row_bytes(row_bytes: &mut usize, added: usize) -> Result<()> {
    *row_bytes = row_bytes
        .checked_add(added)
        .context("materialized row byte count overflow")?;
    if *row_bytes > MAX_MATERIALIZED_ROW_BYTES {
        bail!("materialized row values exceed the 8 MiB limit");
    }
    Ok(())
}

fn cell_text(value: &DataRef<'_>) -> Result<String> {
    match value {
        DataRef::Int(number) => Ok(number.to_string()),
        DataRef::Float(number) => finite_number(*number),
        DataRef::String(text) => Ok(text.clone()),
        DataRef::SharedString(text) => Ok((*text).to_owned()),
        DataRef::Bool(value) => Ok(if *value {
            "1".to_owned()
        } else {
            "0".to_owned()
        }),
        DataRef::DateTime(value) => finite_number(value.as_f64()),
        DataRef::DateTimeIso(text) => Ok(text.clone()),
        DataRef::DurationIso(text) => Ok(text.clone()),
        DataRef::Error(error) => Ok(error.to_string()),
        DataRef::Empty => Ok(String::new()),
    }
}

fn finite_number(number: f64) -> Result<String> {
    if !number.is_finite() {
        bail!("nonfinite numeric cell value cannot be represented");
    }
    Ok(number.to_string())
}
