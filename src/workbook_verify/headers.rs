use super::stream::Row;
use super::MAX_EXCEL_COLUMN;
use crate::workbook_ingest::stream::account_retained_headers;
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;

pub(super) fn validate_extra_headers(extra_headers: &[String]) -> Result<()> {
    if extra_headers.len()
        > usize::try_from(MAX_EXCEL_COLUMN).context("Excel column bound conversion overflow")?
    {
        bail!("extra header count exceeds Excel column limit");
    }
    let mut seen = BTreeSet::new();
    extra_headers.iter().try_for_each(|header| {
        if header.is_empty() || !seen.insert(header) {
            bail!("extra headers must be nonempty and unique");
        }
        Ok(())
    })
}

pub(super) fn parse_headers(row: &Row, retained_header_bytes: &mut usize) -> Result<Vec<String>> {
    let header_bytes = row
        .cells
        .values()
        .filter(|value| !value.is_empty())
        .try_fold(0_usize, |total, value| {
            total
                .checked_add(value.len())
                .context("worksheet header byte count overflow")
        })?;
    account_retained_headers(retained_header_bytes, header_bytes)?;
    let last = row
        .cells
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(column, _)| *column)
        .max()
        .context("worksheet header is empty")?;
    let width = usize::try_from(
        last.checked_add(1)
            .context("worksheet header width overflow")?,
    )
    .context("worksheet header width conversion overflow")?;
    let headers = (0..width)
        .map(|column| {
            let column =
                u32::try_from(column).context("worksheet header column conversion overflow")?;
            Ok(row
                .cells
                .get(&column)
                .map_or_else(String::new, Clone::clone))
        })
        .collect::<Result<Vec<_>>>()?;
    if headers.iter().any(String::is_empty) {
        bail!("worksheet header contains an empty field");
    }
    let mut seen = BTreeSet::new();
    if headers.iter().any(|header| !seen.insert(header)) {
        bail!("worksheet header contains duplicate fields");
    }
    Ok(headers)
}
