//! One cell's contribution to the current row: its shared-string value and its position in the
//! row's column map.

use super::super::super::cells::CellState;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub(super) fn insert_cell(
    cells: &mut BTreeMap<usize, String>,
    column: usize,
    value: String,
) -> Result<()> {
    if cells.insert(column, value).is_some() {
        bail!("worksheet contains duplicate cell position in column {column}");
    }
    Ok(())
}

pub(super) fn resolve_cell_value(cell: &CellState, shared_strings: &[String]) -> Result<String> {
    if cell.cell_type != "s" || cell.value.is_empty() {
        return Ok(cell.value.clone());
    }
    let index = cell
        .value
        .trim()
        .parse::<usize>()
        .with_context(|| format!("invalid shared string index {:?}", cell.value))?;
    match shared_strings.get(index) {
        Some(value) => Ok(value.clone()),
        None => bail!("shared string index {index} is out of range"),
    }
}
