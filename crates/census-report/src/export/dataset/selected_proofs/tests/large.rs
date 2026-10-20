use super::super::objects;
use crate::export::proof_objects;
use crate::report::{ExportResourceError, ReportError};
use sha2::{Digest, Sha256};
use std::io::Write;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn aggregate_proof_survives_source_removal_but_never_bypasses_read_or_integrity_bounds(
) -> TestResult {
    let source = tempfile::tempdir()?;
    let destination = tempfile::tempdir()?;
    let body = vec![0x5a; 8 * 1024 * 1024 + 32];
    let expected_bytes = u64::try_from(body.len())?;
    let digest = proof_objects::install(source.path(), &body)?;
    check!(eq; digest, format!("{:x}", Sha256::digest(&body)));
    let inventory = objects::inventory(source.path())?;
    let [reference] = inventory.as_slice() else {
        return Err("expected one complete aggregate proof".into());
    };
    check!(eq; reference.digest, digest);
    check!(eq; reference.bytes, expected_bytes);
    let small_limit = 8 * 1024 * 1024;
    check!(matches!(
        proof_objects::read(source.path(), &digest, small_limit),
        Err(ReportError::Resource(ExportResourceError::RowTooLarge { bytes, max, .. }))
            if bytes == expected_bytes && max == small_limit
    ));
    let copied = destination.path().join("proofs");
    objects::copy(source.path(), &copied, &inventory)?;
    objects::copy(source.path(), &copied, &inventory)?;
    source.close()?;
    objects::verify(&copied, &inventory)?;
    check!(eq; proof_objects::read(&copied, &digest, expected_bytes)?, body);
    let object = copied.join(&digest);
    let mut file = std::fs::OpenOptions::new().write(true).open(&object)?;
    file.write_all(&[0x4b])?;
    file.sync_all()?;
    drop(file);
    check!(matches!(
        objects::verify(&copied, &inventory),
        Err(ReportError::ArtifactCorrupt { path, .. }) if path == object
    ));
    std::fs::remove_file(&object)?;
    check!(objects::verify(&copied, &inventory).is_err());
    Ok(())
}
