use super::super::archive_io::{
    create_directory, freeze_file, invalid, present, publish_once, sync_directory, write_file,
};
use super::super::capture::Capture;
use super::super::{content_digest, read_cache_file, FetchError, MAX_META_BYTES};
use super::{
    archived_body_path, archived_meta_path, needs_publication, verified_body, ArchiveAdmission,
};
use std::path::{Path, PathBuf};

pub(in crate::net::cache) fn preserve_capture(
    archive: &Path,
    stage: &Path,
    label: &str,
    body: &[u8],
    capture: &Capture,
    admission: &mut ArchiveAdmission<'_>,
) -> Result<PathBuf, FetchError> {
    let digest = &capture.meta.content_digest;
    let metadata_digest = content_digest(&capture.encoded);
    let archived_body = archived_body_path(archive, digest);
    let archived_meta = archived_meta_path(archive, digest, &metadata_digest);
    let body_bytes = publication_bytes(&archived_body, body.len())?;
    let meta_bytes = publication_bytes(&archived_meta, capture.encoded.len())?;
    admission.reserve_capture_sizes(digest, &metadata_digest, body_bytes, meta_bytes)?;
    let result = (|| {
        publish_body(stage, label, body, capture, &archived_body)?;
        let captures = archived_meta
            .parent()
            .ok_or_else(|| invalid(&archived_meta, "capture metadata has no parent"))?;
        create_directory(captures)?;
        publish_metadata(stage, label, &capture.encoded, &archived_meta)?;
        Ok(archived_body)
    })();
    admission.finish_capture(digest, &metadata_digest)?;
    result
}

fn publication_bytes(path: &Path, length: usize) -> Result<Option<u64>, FetchError> {
    if !needs_publication(path)? {
        return Ok(None);
    }
    u64::try_from(length)
        .map(Some)
        .map_err(|_| invalid(path, "capture exceeds the archive accounting range"))
}

fn publish_body(
    stage: &Path,
    label: &str,
    body: &[u8],
    capture: &Capture,
    destination: &Path,
) -> Result<(), FetchError> {
    if !present(destination)? {
        let staged = stage.join(format!("{label}.body"));
        write_file(&staged, body)?;
        freeze_file(&staged)?;
        publish_once(&staged, destination)?;
    }
    if verified_body(destination, capture)? != body {
        return Err(invalid(
            destination,
            "archive body differs from the captured bytes",
        ));
    }
    freeze_file(destination)?;
    let parent = destination
        .parent()
        .ok_or_else(|| invalid(destination, "body has no parent"))?;
    sync_directory(parent)
}

fn publish_metadata(
    stage: &Path,
    label: &str,
    encoded: &[u8],
    destination: &Path,
) -> Result<(), FetchError> {
    if !present(destination)? {
        let staged = stage.join(format!("{label}.meta.json"));
        write_file(&staged, encoded)?;
        freeze_file(&staged)?;
        publish_once(&staged, destination)?;
    }
    verify_metadata(destination, encoded)?;
    freeze_file(destination)?;
    let parent = destination
        .parent()
        .ok_or_else(|| invalid(destination, "metadata has no parent"))?;
    sync_directory(parent)
}

fn verify_metadata(path: &Path, encoded: &[u8]) -> Result<(), FetchError> {
    if !present(path)? {
        return Err(invalid(path, "capture metadata is missing"));
    }
    let stored = read_cache_file(path, MAX_META_BYTES, Some(encoded.len()))?
        .ok_or_else(|| invalid(path, "archive metadata does not match its declared size"))?;
    if stored != encoded {
        return Err(invalid(
            path,
            "archive metadata does not match its content address",
        ));
    }
    Ok(())
}
