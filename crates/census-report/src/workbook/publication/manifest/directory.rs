use super::artifact::{hash_artifact, MAX_BUNDLE_BYTES};
use super::{artifact_names, invariant, sidecars, Manifest};
use crate::report::{io_error, ReportResult};
use std::path::Path;

pub(in crate::workbook::publication) fn verify_directory(
    directory: &Path,
) -> ReportResult<Manifest> {
    let path = directory.join("manifest.json");
    let manifest: Manifest = sidecars::read_json(&path)?;
    if manifest.schema_revision != 1 || manifest.digest()? != manifest.generation_digest {
        return Err(invariant(
            "unsupported or corrupt generation manifest".to_string(),
        ));
    }
    manifest.selection.options()?;
    let expected = artifact_names(manifest.selection.grad_year);
    if manifest.artifacts.keys().cloned().collect::<Vec<_>>() != expected {
        return Err(invariant(
            "generation has an invalid artifact set".to_string(),
        ));
    }
    verify_artifacts(directory, &manifest)?;
    verify_inventory(directory, &manifest)?;
    Ok(manifest)
}

fn verify_artifacts(directory: &Path, manifest: &Manifest) -> ReportResult<()> {
    let mut total = 0_u64;
    manifest.artifacts.iter().try_for_each(|(name, expected)| {
        let actual = hash_artifact(&directory.join(name))?;
        if actual != *expected {
            return Err(invariant(format!("generation artifact mismatch: {name}")));
        }
        total = total
            .checked_add(actual.bytes)
            .ok_or_else(|| invariant("bundle byte counter exhausted".to_string()))?;
        if total > MAX_BUNDLE_BYTES {
            return Err(invariant(
                "generation exceeds 16 GiB byte budget".to_string(),
            ));
        }
        Ok(())
    })
}

fn verify_inventory(directory: &Path, manifest: &Manifest) -> ReportResult<()> {
    let mut entries = std::fs::read_dir(directory).map_err(|source| io_error(directory, source))?;
    entries.try_for_each(|entry| {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let name = entry.file_name();
        if name != "manifest.json"
            && !manifest
                .artifacts
                .contains_key(name.to_string_lossy().as_ref())
        {
            return Err(invariant(format!(
                "unexpected generation artifact: {}",
                name.to_string_lossy()
            )));
        }
        Ok(())
    })
}
