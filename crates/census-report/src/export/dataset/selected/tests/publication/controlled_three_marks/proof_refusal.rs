use super::source::TestResult;
use crate::export::ExportDataset;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn proof_directory(path: &Path) -> TestResult<std::path::PathBuf> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("frozen proof archive has no UTF-8 file name")?;
    let parent = path.parent().ok_or("frozen archive has no parent")?;
    Ok(parent.join(format!("{name}.proofs")))
}

fn required_objects(dataset: &ExportDataset, objects: &Path) -> TestResult<[Option<String>; 3]> {
    let proof = serde_json::to_value(dataset.selected_proofs.as_ref().ok_or("selected proofs")?)?;
    let descriptor = proof
        .get("participation_digest")
        .and_then(Value::as_str)
        .ok_or("participation descriptor")?;
    let replay = crate::export::qualification::participation::ParticipationReplay::load(
        objects, descriptor,
    )?;
    check!(matches!(
        &replay,
        crate::export::qualification::participation::ParticipationReplay::V4(_)
    ));
    let wire = serde_json::to_value(replay)?;
    let required = |pointer: &str| -> TestResult<String> {
        Ok(wire
            .pointer(pointer)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("required V4 closure missing: {pointer}"))?
            .to_owned())
    };
    let calendar = wire
        .get("calendar")
        .map(|_| required("/calendar/object"))
        .transpose()?;
    Ok([
        Some(required("/attachment/object")?),
        Some(required("/school/object")?),
        calendar,
    ])
}

pub(super) fn assert_required_proof_refuses(dataset: &ExportDataset) -> TestResult {
    let frozen = tempfile::tempdir()?;
    let path = frozen.path().join("controlled-proof-refusal.json");
    dataset.save_frozen(&path)?;

    let original_bytes = std::fs::read(&path)?;
    let sha = sha256_bytes(&original_bytes);
    let expected_lineage = dataset.lineage.clone();

    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &sha)?;
    check!(eq; reopened.lineage, expected_lineage);
    drop(reopened);

    let proofs = proof_directory(&path)?;
    for object in required_objects(dataset, &proofs)?.into_iter().flatten() {
        probe(&path, &sha, &proofs.join(object))?;
        let restored = ExportDataset::reopen_frozen_with_digest(&path, &sha)?;
        check!(eq; restored.lineage, expected_lineage);
    }

    check!(eq; std::fs::read(&path)?, original_bytes);

    Ok(())
}

fn probe(path: &Path, sha: &str, closure: &Path) -> TestResult {
    let mut original = std::fs::read(closure)?;
    std::fs::remove_file(closure)?;
    check!(ExportDataset::reopen_frozen_with_digest(path, sha).is_err());
    std::fs::write(closure, &original)?;
    *original.first_mut().ok_or("required proof is empty")? ^= 1;
    std::fs::write(closure, &original)?;
    check!(ExportDataset::reopen_frozen_with_digest(path, sha).is_err());
    *original.first_mut().ok_or("required proof is empty")? ^= 1;
    std::fs::write(closure, original)?;
    Ok(())
}
