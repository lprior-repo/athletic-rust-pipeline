use super::super::sidecars;
use super::artifact::{hash_artifact, MAX_BUNDLE_BYTES};
use super::{artifact_names, invariant, Manifest};
use crate::report::{io_error, ReportResult};
use std::path::Path;

#[derive(serde::Deserialize)]
struct RevisionHead {
    schema_revision: u32,
}

pub(in crate::workbook::publication) fn verify_directory(
    directory: &Path,
) -> ReportResult<Manifest> {
    let path = directory.join("manifest.json");
    let head: RevisionHead = sidecars::read_json(&path)?;
    match head.schema_revision {
        1..=8 | 10..=14 => {}
        _ => {
            return Err(invariant(
                "unsupported publication artifact revision".into(),
            ))
        }
    }
    let manifest: Manifest = sidecars::read_json(&path)?;
    if manifest.schema_revision != head.schema_revision
        || manifest.digest()? != manifest.generation_digest
    {
        return Err(invariant(
            "unsupported or corrupt generation manifest".to_string(),
        ));
    }
    super::super::versions::validate_policy(
        manifest.schema_revision,
        manifest.lineage.policy_revision,
    )?;
    manifest.selection.options()?;
    super::super::inventory::validate(
        manifest.schema_revision,
        manifest.normalized_csv_rows.as_ref(),
    )?;
    let expected = artifact_names(manifest.schema_revision, manifest.selection.grad_year)?;
    if manifest.artifacts.keys().cloned().collect::<Vec<_>>() != expected {
        return Err(invariant(
            "generation has an invalid artifact set".to_string(),
        ));
    }
    let artifact_bytes = verify_artifacts(directory, &manifest)?;
    verify_inventory(directory, &manifest)?;
    if matches!(manifest.schema_revision, 7 | 8 | 10..=14) {
        verify_frozen_proofs(directory, &manifest, artifact_bytes)?;
    }
    Ok(manifest)
}

fn verify_artifacts(directory: &Path, manifest: &Manifest) -> ReportResult<u64> {
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
            return Err(invariant("generation exceeds its byte budget".to_string()));
        }
        Ok(())
    })?;
    Ok(total)
}

fn verify_inventory(directory: &Path, manifest: &Manifest) -> ReportResult<()> {
    let mut entries = std::fs::read_dir(directory).map_err(|source| io_error(directory, source))?;
    entries.try_for_each(|entry| {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let name = entry.file_name();
        if name == "frozen-input.json.proofs" && matches!(manifest.schema_revision, 7 | 8 | 10..=14)
        {
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|source| io_error(&entry.path(), source))?;
            if !metadata.file_type().is_dir() {
                return Err(invariant(
                    "selected proof inventory is not a directory".into(),
                ));
            }
            return Ok(());
        }
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

fn verify_frozen_proofs(
    directory: &Path,
    manifest: &Manifest,
    artifact_bytes: u64,
) -> ReportResult<()> {
    let frozen_path = directory.join("frozen-input.json");
    let proof_bytes = crate::export::ExportDataset::verify_frozen_proof_objects(&frozen_path)?;
    if artifact_bytes
        .checked_add(proof_bytes)
        .is_none_or(|total| total > MAX_BUNDLE_BYTES)
    {
        return Err(invariant(
            "selected generation exceeds its aggregate byte budget".into(),
        ));
    }
    let retained = manifest
        .artifacts
        .get("frozen-input.json")
        .ok_or_else(|| invariant("selected generation lacks frozen input".into()))?;
    if hash_artifact(&frozen_path)? != *retained {
        return Err(invariant(
            "frozen proof inventory changed during verification".into(),
        ));
    }
    Ok(())
}
