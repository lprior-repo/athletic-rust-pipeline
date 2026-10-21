use super::super::invariant;
use super::{directory, Manifest, Selection};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::Options;
use std::path::Path;

pub(super) fn decode(directory: &Path, manifest: &Manifest) -> ReportResult<ExportDataset> {
    let frozen = manifest
        .artifacts
        .get("frozen-input.json")
        .ok_or_else(|| invariant("publication lacks its frozen input".into()))?;
    let dataset = ExportDataset::reopen_frozen_with_digest(
        &directory.join("frozen-input.json"),
        &frozen.sha256,
    )?;
    if dataset.lineage != manifest.lineage {
        return Err(invariant(
            "manifest and frozen input lineage disagree".to_string(),
        ));
    }
    manifest.ensure_dataset(&dataset)?;
    Ok(dataset)
}

pub(in crate::workbook::publication) fn verify_prepared(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> ReportResult<()> {
    let manifest = directory::verify_directory(directory)?;
    manifest.ensure_dataset(dataset)?;
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
    dataset.verify_staged_proof_objects(&directory.join("frozen-input.json"))?;
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
    manifest.ensure_dataset(dataset)?;
    let mut options = manifest.selection.options()?;
    if dataset.selected_profile_id().is_some() && !dataset.is_historical_selected_replay() {
        options.jurisdictions = dataset
            .selected_authority()?
            .profile()
            .input()
            .jurisdictions
            .clone();
        if Selection::of(&options) != manifest.selection {
            return Err(invariant(
                "manifest selection differs from the frozen selected profile".into(),
            ));
        }
    }
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
