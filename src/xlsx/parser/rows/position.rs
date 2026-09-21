//! Excel row numbering: the occurrence fallback for a row without an `r` attribute, and the range
//! and ordering checks every explicit row number has to pass.

use super::super::super::MAX_EXCEL_ROW;
use anyhow::{bail, Context, Result};

pub(super) fn next_row_number(previous: Option<u32>, occurrence: u64) -> Result<u64> {
    previous.map_or(Ok(occurrence), |row| {
        u64::from(row)
            .checked_add(1)
            .context("worksheet row number overflow")
    })
}

pub(super) fn worksheet_row_number(
    value: Option<String>,
    fallback: u64,
    previous: Option<u32>,
) -> Result<u32> {
    let row = value.map_or_else(
        || u32::try_from(fallback).context("worksheet row count exceeds Excel row range"),
        |raw| {
            raw.parse::<u32>()
                .with_context(|| format!("invalid Excel row number {raw:?}"))
        },
    )?;
    if row == 0 || row > MAX_EXCEL_ROW {
        bail!("invalid Excel row number {row}");
    }
    if previous.is_some_and(|prior| row <= prior) {
        bail!("worksheet row position {row} is duplicate or out of order");
    }
    Ok(row)
}
