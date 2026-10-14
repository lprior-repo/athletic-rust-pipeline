use super::archive_io::{
    create_directory, invalid, io_error, lock_cache, present, replace_cache, with_stage,
};
use super::capture::Capture;
use super::quarantine::{quarantine_pair, Damage};
use super::{
    content_digest, read_cache_file, CacheMeta, FetchError, MAX_BODY_BYTES, MAX_META_BYTES,
};
use std::path::{Path, PathBuf};

#[path = "archive/admission.rs"]
pub(super) mod admission;
pub(super) use admission::ArchiveAdmission;
mod lock;
#[cfg(test)]
pub(in crate::net) use lock::tests as lock_tests;
pub(in crate::net) use lock::ArchiveLock;

mod inventory;
mod publication;
pub(super) mod reservations;
pub(super) use publication::preserve_capture;
fn archived_body_path(archive: &Path, content_digest: &str) -> PathBuf {
    archive
        .join("bodies")
        .join(format!("{content_digest}.body"))
}

fn needs_publication(path: &Path) -> Result<bool, FetchError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(false),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(io_error(path, error)),
    }
}

fn archived_meta_path(archive: &Path, content_digest: &str, metadata_digest: &str) -> PathBuf {
    archive
        .join("captures")
        .join(content_digest)
        .join(format!("{metadata_digest}.meta.json"))
}

pub(super) fn write_preserved_cache(
    body_path: &Path,
    meta_path: &Path,
    body: &[u8],
    meta: &CacheMeta,
) -> Result<(), FetchError> {
    let root = cache_root(body_path, meta_path)?;
    let capacity = admission::archive_capacity()?;
    let capture = Capture::from_meta(meta, meta_path)?;
    capture.verify_body(body, body_path)?;
    let archive_lock = ArchiveLock::acquire(root)?;
    let archive = initialize_archive(root)?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock, capacity)?;
    with_stage(root, |stage| {
        let lock = match lock_cache(meta_path) {
            Ok(lock) => lock,
            Err(error) => {
                preserve_capture(&archive, stage, "new", body, &capture, &mut admission)?;
                return Err(error);
            }
        };
        preserve_previous(&archive, stage, body_path, meta_path, &mut admission)?;
        let archived_body =
            preserve_capture(&archive, stage, "new", body, &capture, &mut admission)?;
        let result = replace_cache(
            stage,
            &archived_body,
            body_path,
            meta_path,
            &capture.encoded,
        );
        drop(lock);
        result
    })
}

pub(super) fn write_preserved_capture(
    body_path: &Path,
    meta_path: &Path,
    body: &[u8],
    meta: &CacheMeta,
) -> Result<PathBuf, FetchError> {
    let root = cache_root(body_path, meta_path)?;
    let capacity = admission::archive_capacity()?;
    let capture = Capture::from_meta(meta, meta_path)?;
    capture.verify_body(body, body_path)?;
    let archive_lock = ArchiveLock::acquire(root)?;
    let archive = initialize_archive(root)?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock, capacity)?;
    with_stage(root, |stage| {
        preserve_capture(&archive, stage, "new", body, &capture, &mut admission)
    })
}

pub(super) fn replay_preserved_cache(
    body_path: &Path,
    meta_path: &Path,
    expected: &CacheMeta,
) -> Result<Option<Vec<u8>>, FetchError> {
    let root = cache_root(body_path, meta_path)?;
    let capacity = admission::archive_capacity()?;
    let archive_lock = ArchiveLock::acquire(root)?;
    let lock = lock_cache(meta_path)?;
    let Some((meta, body)) = super::read_cache(
        body_path,
        meta_path,
        &expected.method,
        &expected.url,
        &expected.representation,
    )?
    else {
        return Ok(None);
    };
    if &meta != expected {
        return Ok(None);
    }
    let archive = initialize_archive(root)?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock, capacity)?;
    with_stage(root, |stage| {
        preserve_previous(&archive, stage, body_path, meta_path, &mut admission)
    })?;
    drop(lock);
    Ok(Some(body))
}

pub(super) fn cache_root<'a>(
    body_path: &'a Path,
    meta_path: &Path,
) -> Result<&'a Path, FetchError> {
    if body_path == meta_path || body_path.file_name().is_none() || meta_path.file_name().is_none()
    {
        return Err(invalid(
            body_path,
            "cache body and metadata require distinct file paths",
        ));
    }
    let root = body_path
        .parent()
        .ok_or_else(|| invalid(body_path, "cache body has no parent"))?;
    if meta_path.parent() != Some(root) {
        return Err(invalid(
            meta_path,
            "cache body and metadata must share a directory",
        ));
    }
    if root.as_os_str().is_empty() {
        Ok(Path::new("."))
    } else {
        Ok(root)
    }
}

pub(super) fn initialize_archive(root: &Path) -> Result<PathBuf, FetchError> {
    let archive = root.join("archive");
    create_directory(&archive)?;
    create_directory(&archive.join("bodies"))?;
    create_directory(&archive.join("captures"))?;
    Ok(archive)
}

fn preserve_previous(
    archive: &Path,
    stage: &Path,
    body_path: &Path,
    meta_path: &Path,
    admission: &mut ArchiveAdmission<'_>,
) -> Result<(), FetchError> {
    let presence = (present(body_path)?, present(meta_path)?);
    match presence {
        (false, false) => return Ok(()),
        (true, true) => {}
        _ => {
            return quarantine_pair(
                stage,
                body_path,
                meta_path,
                presence,
                Damage::IncompletePair,
            )
        }
    }
    match read_previous(body_path, meta_path)? {
        Ok((capture, body)) => {
            preserve_capture(archive, stage, "old", &body, &capture, admission)?;
        }
        Err(reason) => {
            quarantine_pair(stage, body_path, meta_path, presence, reason)?;
        }
    }
    Ok(())
}

type Previous = Result<(Capture, Vec<u8>), Damage>;

fn read_previous(body_path: &Path, meta_path: &Path) -> Result<Previous, FetchError> {
    let Some(encoded) = read_cache_file(meta_path, MAX_META_BYTES, None)? else {
        return Ok(Err(Damage::MetadataLimitOrChanged));
    };
    let decoded =
        super::envelope::decode(&encoded, meta_path).and_then(|document| match document {
            super::envelope::CacheDocument::Historical(_) => {
                Capture::from_bytes(&encoded, meta_path)
            }
            super::envelope::CacheDocument::Eligible(envelope) => {
                Capture::from_meta(&envelope.metadata, meta_path)
            }
        });
    let capture = match decoded {
        Ok(capture) => capture,
        Err(FetchError::Decode { .. } | FetchError::Encode { .. }) => {
            return Ok(Err(Damage::InvalidMetadata));
        }
        Err(FetchError::Cache { source, .. })
            if source.kind() == std::io::ErrorKind::InvalidData =>
        {
            return Ok(Err(Damage::InvalidMetadata));
        }
        Err(error) => return Err(error),
    };
    let Some(body) = read_cache_file(body_path, MAX_BODY_BYTES, Some(capture.meta.bytes))? else {
        return Ok(Err(Damage::BodySizeOrChanged));
    };
    if content_digest(&body) != capture.meta.content_digest {
        return Ok(Err(Damage::BodyIntegrity));
    }
    Ok(Ok((capture, body)))
}

fn verified_body(path: &Path, capture: &Capture) -> Result<Vec<u8>, FetchError> {
    if !present(path)? {
        return Err(invalid(path, "capture body is missing"));
    }
    let body =
        read_cache_file(path, MAX_BODY_BYTES, Some(capture.meta.bytes))?.ok_or_else(|| {
            invalid(
                path,
                "capture body exceeds its limit or does not match its declared size",
            )
        })?;
    capture.verify_body(&body, path)?;
    Ok(body)
}

pub(super) fn preserve_eligible_previous(
    archive: &Path,
    stage: &Path,
    body_path: &Path,
    meta_path: &Path,
    admission: &mut ArchiveAdmission<'_>,
) -> Result<(), FetchError> {
    preserve_previous(archive, stage, body_path, meta_path, admission)
}
