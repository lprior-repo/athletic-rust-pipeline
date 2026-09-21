use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate_named_columns(
    cells: &BTreeMap<u32, String>,
    headers: &[String],
) -> Result<()> {
    cells.iter().try_for_each(|(column, value)| {
        let index = usize::try_from(*column).context("worksheet column conversion overflow")?;
        if !value.is_empty() && index >= headers.len() {
            bail!("worksheet data appears in unnamed column {column}");
        }
        Ok(())
    })
}

pub(super) fn build_headers(cells: &BTreeMap<u32, String>) -> Result<Vec<String>> {
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
