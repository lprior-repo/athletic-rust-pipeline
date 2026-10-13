use super::super::codec::{count, sha256_hex};
use super::super::fixtures::TestResult;
use crate::report::{CorruptionClass, ExportResourceError, ReportError};

#[test]
fn physical_resource_preservation_read_path_reports_observed_size_at_ceiling_plus_one() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("bounded-read");
    let bytes = vec![b'x'; 17];
    std::fs::write(&path, &bytes)?;
    let error = match super::read_path(&path, 16) {
        Ok(_) => return Err("read accepted ceiling plus one".into()),
        Err(error) => error,
    };
    check!(matches!(
        error,
        ReportError::Resource(ExportResourceError::RowTooLarge {
            what: "frozen object",
            bytes: 17,
            max: 16,
        })
    ));
    Ok(())
}

#[test]
fn physical_resource_preservation_read_path_stops_at_ceiling_plus_one() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("bounded-read");
    std::fs::write(&path, vec![b'x'; 4096])?;
    let error = match super::read_path(&path, 16) {
        Ok(_) => return Err("read accepted an oversized object".into()),
        Err(error) => error,
    };
    check!(matches!(
        error,
        ReportError::Resource(ExportResourceError::RowTooLarge {
            what: "frozen object",
            bytes: 17,
            max: 16,
        })
    ));
    Ok(())
}

#[test]
fn physical_resource_preservation_nonregular_files_and_wrong_hashes_remain_corruption() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let digest = sha256_hex(b"expected");
    let path = directory.path().join(&digest);
    std::fs::create_dir(&path)?;
    let nonregular = match super::read(directory.path(), &digest, 16) {
        Ok(_) => return Err("directory object was read".into()),
        Err(error) => error,
    };
    check!(eq; nonregular.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    check!(eq; nonregular.resource(), None);
    check!(
        matches!(nonregular, ReportError::ArtifactCorrupt { path: actual, detail }
        if actual == path && detail == "a frozen object must be a regular file")
    );
    std::fs::remove_dir(&path)?;
    std::fs::write(&path, b"changed")?;
    let wrong_hash = match super::read(directory.path(), &digest, 16) {
        Ok(_) => return Err("wrong-hash object was read".into()),
        Err(error) => error,
    };
    check!(eq; wrong_hash.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    check!(eq; wrong_hash.resource(), None);
    check!(
        matches!(wrong_hash, ReportError::ArtifactCorrupt { path: actual, detail }
        if actual == path && detail == "the frozen object does not hash to its name")
    );
    Ok(())
}

#[test]
fn physical_resource_preservation_invalid_object_names_keep_the_existing_invariant_refusal(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let error = match super::read(directory.path(), "bad", 16) {
        Ok(_) => return Err("invalid object name was accepted".into()),
        Err(error) => error,
    };
    check!(eq; error.resource(), None);
    check!(matches!(error, ReportError::Invariant { detail }
        if detail == "frozen object name bad is not a lowercase sha256 digest"));
    check!(eq; count(std::fs::read_dir(directory.path())?.count())?, 0);
    Ok(())
}
