use anyhow::{bail, Context, Result};

pub(crate) const MAX_MATERIALIZED_ROW_BYTES: usize = 8 * 1024 * 1024;
pub(super) const MAX_RETAINED_HEADER_BYTES: usize = 1024 * 1024;

pub(crate) fn account_retained_headers(retained: &mut usize, added: usize) -> Result<()> {
    *retained = retained
        .checked_add(added)
        .context("retained workbook header byte count overflow")?;
    if *retained > MAX_RETAINED_HEADER_BYTES {
        bail!("retained workbook headers exceed the 1 MiB limit");
    }
    Ok(())
}

pub(super) fn account_row_bytes(row_bytes: &mut usize, added: usize) -> Result<()> {
    *row_bytes = row_bytes
        .checked_add(added)
        .context("materialized row byte count overflow")?;
    if *row_bytes > MAX_MATERIALIZED_ROW_BYTES {
        bail!("materialized row values exceed the 8 MiB limit");
    }
    Ok(())
}
