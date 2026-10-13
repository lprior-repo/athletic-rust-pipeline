use super::super::codec::{count, sha256_hex};
use super::super::fixtures::TestResult;
use crate::report::ReportError;

fn no_replace(error: ReportError, path: &std::path::Path, digest: &str) -> TestResult {
    check!(eq; error.resource(), None);
    check!(eq; error.corruption(), None);
    let expected = format!(
        "frozen object {} already exists with different bytes; refusing to replace {digest}",
        path.display(),
    );
    check!(matches!(error, ReportError::Invariant { detail } if detail == expected));
    Ok(())
}

#[test]
fn physical_resource_preservation_expected_length_growth_retains_no_replace_refusal() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let original = b"immutable bytes";
    let digest = sha256_hex(original);
    let path = directory.path().join(&digest);
    std::fs::write(&path, original)?;
    let length = count(original.len())?;
    let observed = std::fs::symlink_metadata(&path)?;
    check!(eq; observed.len(), length);
    let mut changed = original.to_vec();
    changed.push(b'x');
    std::fs::write(&path, &changed)?;
    let error = match super::read_expected_length(&path, &digest, length) {
        Ok(_) => return Err("growth beyond immutable expected length was accepted".into()),
        Err(error) => error,
    };
    no_replace(error, &path, &digest)?;
    check!(eq; std::fs::read(&path)?, changed);
    Ok(())
}

#[test]
fn physical_resource_preservation_expected_length_and_digest_reuse_remain_exact() -> TestResult {
    let directory = tempfile::tempdir()?;
    let original = b"immutable bytes";
    let digest = sha256_hex(original);
    let path = directory.path().join(&digest);
    let length = count(original.len())?;
    std::fs::write(&path, original)?;
    check!(eq; super::read_expected_length(&path, &digest, length)?, original);
    super::reuse(&path, &digest, length)?;
    check!(eq; std::fs::read(&path)?, original);
    let changed = vec![b'x'; original.len()];
    std::fs::write(&path, &changed)?;
    let error = match super::reuse(&path, &digest, length) {
        Ok(()) => return Err("same-length wrong-hash immutable object was reused".into()),
        Err(error) => error,
    };
    no_replace(error, &path, &digest)?;
    check!(eq; std::fs::read(&path)?, changed);
    Ok(())
}

#[test]
fn physical_resource_preservation_expected_length_read_preserves_io_failures() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("absent");
    let digest = sha256_hex(b"immutable bytes");
    let error = match super::read_expected_length(&path, &digest, 15) {
        Ok(_) => return Err("missing immutable object was read".into()),
        Err(error) => error,
    };
    check!(eq; error.resource(), None);
    check!(matches!(error, ReportError::Io { path: actual, source }
        if actual == path && source.kind() == std::io::ErrorKind::NotFound));
    Ok(())
}

#[test]
fn physical_resource_preservation_expected_length_read_preserves_other_failures() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("present");
    let original = b"immutable bytes";
    let digest = sha256_hex(original);
    std::fs::write(&path, original)?;
    let error = match super::read_expected_length(&path, &digest, u64::MAX) {
        Ok(_) => return Err("overflowed read ceiling was accepted".into()),
        Err(error) => error,
    };
    check!(eq; error.resource(), None);
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == "a frozen object ceiling overflowed u64"));
    check!(eq; std::fs::read(&path)?, original);
    Ok(())
}
