use super::archive_io::{
    create_directory, freeze_file, invalid, io_error, lock_cache, present, publish_once,
    replace_cache, sync_directory, with_stage, write_file,
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
    let capture = Capture::from_meta(meta, meta_path)?;
    capture.verify_body(body, body_path)?;
    let archive = initialize_archive(root)?;
    let mut admission = ArchiveAdmission::begin(root, &archive)?;
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
    let capture = Capture::from_meta(meta, meta_path)?;
    capture.verify_body(body, body_path)?;
    let archive = initialize_archive(root)?;
    let mut admission = ArchiveAdmission::begin(root, &archive)?;
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
    let mut admission = ArchiveAdmission::begin(root, &archive)?;
    with_stage(root, |stage| {
        preserve_previous(&archive, stage, body_path, meta_path, &mut admission)
    })?;
    drop(lock);
    Ok(Some(body))
}

fn cache_root<'a>(body_path: &'a Path, meta_path: &Path) -> Result<&'a Path, FetchError> {
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
    admission: &mut ArchiveAdmission,
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
    let capture = match Capture::from_bytes(&encoded, meta_path) {
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

pub(super) fn preserve_capture(
    archive: &Path,
    stage: &Path,
    label: &str,
    body: &[u8],
    capture: &Capture,
    admission: &mut ArchiveAdmission,
) -> Result<PathBuf, FetchError> {
    let digest = &capture.meta.content_digest;
    let archived_body = archived_body_path(archive, digest);
    let body_present = present(&archived_body)?;
    let captures = archive.join("captures").join(digest);
    let metadata_digest = content_digest(&capture.encoded);
    let archived_meta = archived_meta_path(archive, digest, &metadata_digest);
    let meta_needed = needs_publication(&archived_meta)?;
    let body_bytes = u64::try_from(body.len())
        .map_err(|_| invalid(archive, "capture body exceeds the archive accounting range"))?;
    let meta_bytes = u64::try_from(capture.encoded.len()).map_err(|_| {
        invalid(
            archive,
            "capture metadata exceeds the archive accounting range",
        )
    })?;
    let reserved_body = (!body_present).then_some(body_bytes);
    let reserved_meta = meta_needed.then_some(meta_bytes);
    admission.reserve_capture_sizes(digest, &metadata_digest, reserved_body, reserved_meta)?;
    if !body_present {
        let staged_body = stage.join(format!("{label}.body"));
        write_file(&staged_body, body)?;
        freeze_file(&staged_body)?;
        publish_once(&staged_body, &archived_body)?;
    }
    if verified_body(&archived_body, capture)? != body {
        admission.release_capture_sizes(digest, &metadata_digest, reserved_body, reserved_meta)?;
        return Err(invalid(
            &archived_body,
            "archive body differs from the captured bytes",
        ));
    }
    freeze_file(&archived_body)?;
    sync_directory(&archive.join("bodies"))?;
    create_directory(&captures)?;
    if !present(&archived_meta)? {
        let staged_meta = stage.join(format!("{label}.meta.json"));
        write_file(&staged_meta, &capture.encoded)?;
        freeze_file(&staged_meta)?;
        publish_once(&staged_meta, &archived_meta)?;
    }
    if let Err(error) = verify_metadata(&archived_meta, &capture.encoded) {
        admission.release_capture_sizes(digest, &metadata_digest, reserved_body, reserved_meta)?;
        return Err(error);
    }
    freeze_file(&archived_meta)?;
    sync_directory(&captures)?;
    admission.settle_capture_sizes(digest, &metadata_digest, reserved_body, reserved_meta)?;
    Ok(archived_body)
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
