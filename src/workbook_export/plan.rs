use super::sheet::{SheetState, MAX_EXCEL_COLUMN};
use anyhow::{bail, Result};
use std::collections::BTreeSet;

pub(super) fn validate_extra_headers(
    extra_headers: &[String],
    sheets: &[crate::model::SheetStats],
) -> Result<Vec<String>> {
    if extra_headers.len() > MAX_EXCEL_COLUMN {
        bail!("extra header count exceeds Excel column limit");
    }
    let mut seen = BTreeSet::new();
    extra_headers.iter().try_for_each(|header| {
        if header.is_empty() || !seen.insert(header) {
            bail!("extra headers must be nonempty and unique");
        }
        if sheets
            .iter()
            .any(|sheet| sheet.headers.iter().any(|source| source == header))
        {
            bail!("extra header collides with a source header: {header:?}");
        }
        Ok(())
    })?;
    Ok(extra_headers.to_owned())
}

pub(super) fn build_sheet_states(
    stats: &[crate::model::SheetStats],
    extra_headers: &[String],
) -> Result<Vec<SheetState>> {
    if stats.is_empty() {
        bail!("source manifest contains no worksheets");
    }
    let mut names = BTreeSet::new();
    stats
        .iter()
        .map(|sheet| {
            if !names.insert(sheet.name.clone()) {
                bail!("duplicate source worksheet {:?}", sheet.name);
            }
            validate_source_headers(sheet)?;
            if sheet
                .headers
                .len()
                .checked_add(extra_headers.len())
                .is_none_or(|width| width > MAX_EXCEL_COLUMN)
            {
                bail!("worksheet {:?} exceeds Excel column limit", sheet.name);
            }
            let mut headers = sheet.headers.clone();
            headers.extend(extra_headers.iter().cloned());
            Ok(SheetState {
                name: sheet.name.clone(),
                source_headers: sheet.headers.clone(),
                headers,
                expected_rows: sheet.actual_data_rows,
                expected_last_row: sheet.last_actual_row,
                written_rows: 0,
                last_written_row: None,
            })
        })
        .collect()
}

fn validate_source_headers(sheet: &crate::model::SheetStats) -> Result<()> {
    let mut headers = BTreeSet::new();
    sheet.headers.iter().try_for_each(|header| {
        if header.is_empty() || !headers.insert(header) {
            bail!("worksheet {:?} has empty or duplicate headers", sheet.name);
        }
        Ok(())
    })
}
