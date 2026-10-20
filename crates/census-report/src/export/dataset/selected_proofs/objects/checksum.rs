use super::Reference;
use crate::report::file_digest::{hash_reader, FileDigestError};
use crate::report::{io_error, ReportError, ReportResult};
use std::path::Path;

pub(super) fn verify(directory: &Path, reference: &Reference) -> ReportResult<()> {
    let path = directory.join(&reference.digest);
    let metadata = std::fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
    if !metadata.file_type().is_file() || metadata.len() != reference.bytes {
        return Err(corrupt(
            &path,
            "selected proof is not an exact regular object",
        ));
    }
    let mut file = std::fs::File::open(&path).map_err(|source| io_error(&path, source))?;
    let actual = hash_reader(&mut file, reference.bytes).map_err(|error| match error {
        FileDigestError::Io(source) => io_error(&path, source),
        other => corrupt(&path, &other.to_string()),
    })?;
    if actual.bytes != reference.bytes || actual.sha256 != reference.digest {
        return Err(corrupt(
            &path,
            "selected proof differs from its exact length or digest",
        ));
    }
    Ok(())
}

fn corrupt(path: &Path, detail: &str) -> ReportError {
    ReportError::ArtifactCorrupt {
        path: path.to_path_buf(),
        detail: detail.to_string(),
    }
}
