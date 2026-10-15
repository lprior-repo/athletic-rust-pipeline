use super::super::invariant;
use super::{directory, Manifest, Selection};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::Options;
use std::path::Path;

pub(super) fn decode(directory: &Path, manifest: &Manifest) -> ReportResult<ExportDataset> {
    let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
    if dataset.lineage != manifest.lineage {
        return Err(invariant(
            "manifest and frozen input lineage disagree".to_string(),
        ));
    }
    Ok(dataset)
}

pub(in crate::workbook::publication) fn verify_prepared(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> ReportResult<()> {
    let manifest = directory::verify_directory(directory)?;
    if manifest.lineage != dataset.lineage || manifest.selection != Selection::of(options) {
        return Err(invariant(
            "prepared publication differs from its captured input or selection".to_string(),
        ));
    }
    let frozen = manifest
        .artifacts
        .get("frozen-input.json")
        .ok_or_else(|| invariant("publication lacks its frozen input".to_string()))?;
    if frozen.sha256 != dataset.archive_digest()? {
        return Err(invariant(
            "retained frozen bytes differ from the captured export input".to_string(),
        ));
    }
    let counts = super::super::verify_sidecars::verify(
        directory,
        dataset,
        options,
        manifest.schema_revision,
    )?;
    super::super::inventory::compare(
        manifest.schema_revision,
        manifest.normalized_csv_rows.as_ref(),
        &counts,
    )?;
    crate::workbook::verify::verify_frozen(&directory.join("workbook.xlsx"), dataset, options)?;
    Ok(())
}

pub(super) fn preflight(path: &Path) -> ReportResult<(std::path::PathBuf, Manifest)> {
    if path.file_name().and_then(|name| name.to_str()) != Some("workbook.xlsx") {
        return Err(invariant(
            "verification requires a manifested generation workbook".to_string(),
        ));
    }
    let directory = path
        .parent()
        .ok_or_else(|| invariant("workbook has no generation directory".to_string()))?;
    let manifest = directory::verify_directory(directory)?;
    Ok((directory.to_path_buf(), manifest))
}

pub(super) fn verify_against(
    directory: &Path,
    manifest: &Manifest,
    dataset: &ExportDataset,
) -> ReportResult<crate::workbook::verify::VerifiedWorkbook> {
    let options = manifest.selection.options()?;
    let counts = super::super::verify_sidecars::verify(
        directory,
        dataset,
        &options,
        manifest.schema_revision,
    )?;
    super::super::inventory::compare(
        manifest.schema_revision,
        manifest.normalized_csv_rows.as_ref(),
        &counts,
    )?;
    crate::workbook::verify::verify_frozen(&directory.join("workbook.xlsx"), dataset, &options)
}
