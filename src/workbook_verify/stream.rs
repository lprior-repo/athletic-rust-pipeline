use anyhow::{bail, Context, Result};
use calamine::{Cell, DataRef};
use std::collections::BTreeMap;

const MAX_EXCEL_ROW: u32 = 1_048_576;
const MAX_EXCEL_COLUMN: u32 = 16_384;

#[derive(Debug, Clone)]
pub(crate) struct CellEvent {
    row: u32,
    column: u32,
    value: String,
}

#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub(crate) number: u32,
    pub(crate) cells: BTreeMap<u32, String>,
}

impl Row {
    fn has_data(&self) -> bool {
        self.cells.values().any(|value| !value.is_empty())
    }
}

pub(crate) struct CellRows<F> {
    next: F,
    pending: Option<CellEvent>,
    exhausted: bool,
}

fn retain_cell(
    cells: &mut BTreeMap<u32, String>,
    row_bytes: &mut usize,
    event: CellEvent,
) -> Result<()> {
    *row_bytes = row_bytes
        .checked_add(event.value.len())
        .context("materialized row byte count overflow")?;
    if *row_bytes > crate::workbook_ingest::stream::MAX_MATERIALIZED_ROW_BYTES {
        bail!("materialized row values exceed the 8 MiB limit");
    }
    if cells.insert(event.column, event.value).is_some() {
        bail!("worksheet contains duplicate cell positions");
    }
    Ok(())
}

impl<F> CellRows<F>
where
    F: FnMut() -> Result<Option<CellEvent>>,
{
    pub(crate) fn new(next: F) -> Self {
        Self {
            next,
            pending: None,
            exhausted: false,
        }
    }

    fn next_row(&mut self) -> Result<Option<Row>> {
        if self.exhausted && self.pending.is_none() {
            return Ok(None);
        }
        let first = match self.pending.take() {
            Some(event) => event,
            None => match (self.next)()? {
                Some(event) => event,
                None => {
                    self.exhausted = true;
                    return Ok(None);
                }
            },
        };
        let row_number = first.row;
        let mut cells = BTreeMap::new();
        let mut row_bytes = 0_usize;
        retain_cell(&mut cells, &mut row_bytes, first)?;
        let mut finished = false;
        std::iter::from_fn(|| {
            if finished {
                return None;
            }
            match (self.next)() {
                Ok(Some(event)) if event.row == row_number => Some(Ok(event)),
                Ok(Some(event)) => {
                    self.pending = Some(event);
                    finished = true;
                    None
                }
                Ok(None) => {
                    self.exhausted = true;
                    finished = true;
                    None
                }
                Err(error) => {
                    self.exhausted = true;
                    finished = true;
                    Some(Err(error))
                }
            }
        })
        .try_for_each(|event| retain_cell(&mut cells, &mut row_bytes, event?))?;
        Ok(Some(Row {
            number: row_number,
            cells,
        }))
    }
}

impl<F> Iterator for CellRows<F>
where
    F: FnMut() -> Result<Option<CellEvent>>,
{
    type Item = Result<Row>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.next_row() {
                Ok(Some(row)) if row.has_data() => return Some(Ok(row)),
                Ok(Some(_)) => {}
                Ok(None) => return None,
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

pub(crate) fn cell_event(cell: Cell<DataRef<'_>>) -> Result<CellEvent> {
    let (row, column) = cell.get_position();
    if row >= MAX_EXCEL_ROW || column >= MAX_EXCEL_COLUMN {
        bail!("cell position exceeds Excel bounds");
    }
    Ok(CellEvent {
        row,
        column,
        value: cell_text(cell.get_value())?,
    })
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
