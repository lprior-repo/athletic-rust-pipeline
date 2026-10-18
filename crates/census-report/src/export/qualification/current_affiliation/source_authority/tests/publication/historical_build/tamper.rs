use super::*;
use authentication::{relative, string};
use tempfile::TempDir;

pub(super) fn required_object_refuses(retained: &authentication::Retained) -> TestResult {
    let resolved = retained
        .manifest
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("retained cases")?
        .iter()
        .find(|case| case.get("case").and_then(Value::as_str) == Some("resolved"))
        .ok_or("resolved case not retained")?;
    let original_archive = relative(&retained.root, string(resolved, "archive")?)?;
    let archive_name = original_archive
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("original archive file name")?;
    let original_digest = string(resolved, "archive_digest")?;
    let temporary = TempDir::new()?;
    let copied = temporary.path().join("archive");
    files::copy(original_archive.parent().ok_or("archive parent")?, &copied)?;
    let archive = copied.join(archive_name);
    let wire: Value = serde_json::from_slice(&files::read(&archive)?)?;
    let reference = wire
        .pointer("/selected_proofs/affiliation/archives")
        .ok_or("required affiliation closure")?;
    let digest = string(reference, "digest")?;
    check!(
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    let object = archive
        .with_file_name(format!("{archive_name}.proofs"))
        .join(digest);
    let mut bytes = files::read(&object)?;
    *bytes
        .first_mut()
        .ok_or("required affiliation object is empty")? ^= 1;
    std::fs::write(&object, bytes)?;
    let error = match ExportDataset::reopen_frozen_with_digest(&archive, original_digest) {
        Ok(_) => return Err("tampered required object accepted by frozen archive".into()),
        Err(error) => error,
    };
    check!(matches!(
        error,
        ReportError::ArtifactCorrupt { path, detail }
            if path == object && detail == "the frozen object does not hash to its name"
    ));
    Ok(())
}
