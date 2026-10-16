use crate::export::ExportDataset;
use crate::report::{io_error, ReportError};
use census_store::Store;
use sha2::{Digest, Sha256};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn archive(directory: &Path) -> TestResult<(ExportDataset, std::path::PathBuf, Vec<u8>)> {
    let store = Store::open(directory.join("store"))?;
    let dataset = ExportDataset::load(&store)?;
    let path = directory.join("frozen.json");
    dataset.save_frozen(&path)?;
    let bytes = std::fs::read(&path)?;
    Ok((dataset, path, bytes))
}

fn byte_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn commitment_refused(result: crate::report::ReportResult<ExportDataset>) -> TestResult {
    check!(matches!(
        result,
        Err(ReportError::Invariant { ref detail })
            if detail == "frozen archive differs from its external commitment"
    ));
    Ok(())
}

fn formatting_commitment(original: &[u8], modified: &[u8], path: &Path) -> TestResult {
    check!(ne; original, modified);
    let expected = byte_digest(original);
    check!(eq; ExportDataset::reopen_frozen_with_digest(path, &expected)?.archive_digest()?, expected);
    std::fs::write(path, modified)?;
    let unbound = ExportDataset::reopen_frozen(path)?;
    commitment_refused(ExportDataset::reopen_frozen_with_digest(path, &expected))?;
    let rebound = ExportDataset::reopen_frozen_with_digest(path, &byte_digest(modified))?;
    check!(eq; rebound.lineage, unbound.lineage);
    check!(eq; std::fs::read(path)?, modified);
    Ok(())
}

#[test]
fn whitespace_change_requires_a_new_exact_byte_commitment() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (_, path, original) = archive(directory.path())?;
    let value: serde_json::Value = serde_json::from_slice(&original)?;
    let modified = serde_json::to_vec_pretty(&value)?;
    formatting_commitment(&original, &modified, &path)
}

#[test]
fn key_order_change_requires_a_new_exact_byte_commitment() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (_, path, original) = archive(directory.path())?;
    let value: serde_json::Value = serde_json::from_slice(&original)?;
    let modified = serde_json::to_vec(&value)?;
    check!(eq; original.len(), modified.len());
    formatting_commitment(&original, &modified, &path)
}

#[test]
fn wrong_commitment_is_refused_before_decoding_invalid_json() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("malformed.json");
    std::fs::write(&path, b"not JSON")?;
    commitment_refused(ExportDataset::reopen_frozen_with_digest(
        &path,
        &byte_digest(b"{}"),
    ))
}

#[test]
fn same_length_archive_change_during_preflight_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (_, path, original) = archive(directory.path())?;
    let value: serde_json::Value = serde_json::from_slice(&original)?;
    let modified = serde_json::to_vec(&value)?;
    check!(eq; original.len(), modified.len());
    check!(ne; original, modified);
    let result = super::reopen(&path, Some(&byte_digest(&original)), |_| {
        std::fs::write(&path, &modified).map_err(|source| io_error(&path, source))
    });
    commitment_refused(result)?;
    check!(eq; std::fs::read(&path)?, modified);
    Ok(())
}

#[test]
fn oversized_sparse_archive_is_refused_before_any_hash_or_decode() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("oversized.json");
    let bytes = crate::export::MAX_FROZEN_INPUT_BYTES
        .checked_add(1)
        .ok_or("fixture byte count exhausted")?;
    std::fs::File::create(&path)?.set_len(bytes)?;
    check!(matches!(
        ExportDataset::reopen_frozen_with_digest(&path, &byte_digest(b"{}")),
        Err(ReportError::Invariant { ref detail })
            if detail == &format!("invalid or oversized frozen input {}", path.display())
    ));
    check!(eq; std::fs::metadata(&path)?.len(), bytes);
    Ok(())
}

#[cfg(unix)]
#[test]
fn committed_archive_cannot_be_opened_through_a_symlink() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (_, path, original) = archive(directory.path())?;
    let link = directory.path().join("linked.json");
    std::os::unix::fs::symlink(&path, &link)?;
    check!(matches!(
        ExportDataset::reopen_frozen_with_digest(&link, &byte_digest(&original)),
        Err(ReportError::Invariant { ref detail })
            if detail == &format!("invalid or oversized frozen input {}", link.display())
    ));
    Ok(())
}
