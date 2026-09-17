mod guards;
mod preflight;
pub(crate) mod stream;
use crate::{
    model::{SourceRecord, WorkbookStats},
    xlsx::metadata::load_sheet_metadata,
};
use anyhow::{bail, Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use std::{fs::File, io::BufReader, path::Path};

pub(crate) fn preflight(path: &Path) -> Result<()> {
    preflight::inspect(path).map(|_| ())
}

fn is_source_sheet(name: &str) -> bool {
    !["Athletic Matches", "Corrections", "Summary"]
        .iter()
        .any(|generated| name.eq_ignore_ascii_case(generated))
}

pub fn visit_records(
    path: &Path,
    mut callback: impl FnMut(SourceRecord) -> Result<()>,
) -> Result<WorkbookStats> {
    let preflight = preflight::inspect(path)?;
    // Calamine exposes workbook-order names and cell readers, but not the raw
    // OOXML worksheet path needed to join preflight dimensions and row counts.
    // Keep the existing metadata reader only for that narrow path mapping.
    let metadata = load_sheet_metadata(path)?;
    let mut workbook: Xlsx<BufReader<File>> =
        open_workbook(path).context("open workbook with calamine")?;
    let source_sheets = workbook
        .sheet_names()
        .into_iter()
        .filter(|name| is_source_sheet(name))
        .collect::<Vec<_>>();
    if source_sheets.is_empty() {
        bail!("source workbook contains no source worksheets");
    }
    let mut retained_header_bytes = 0_usize;
    source_sheets
        .into_iter()
        .try_fold(WorkbookStats::default(), |mut stats, name| {
            let sheet = metadata
                .iter()
                .find(|sheet| sheet.name == name.as_str())
                .with_context(|| format!("worksheet metadata missing for {name}"))?;
            let bounds = preflight
                .worksheets
                .iter()
                .find(|item| item.path == sheet.path)
                .with_context(|| format!("worksheet XML missing from preflight: {}", sheet.path))?;
            let sheet_stats = stream::visit_sheet(
                &mut workbook,
                name.as_str(),
                bounds.declared_dimension.clone(),
                bounds.xml_rows,
                &mut retained_header_bytes,
                &mut callback,
            )?;
            stats.actual_data_rows = stats
                .actual_data_rows
                .checked_add(sheet_stats.actual_data_rows)
                .context("workbook data row count overflow")?;
            stats.sheets.push(sheet_stats);
            Ok(stats)
        })
}
