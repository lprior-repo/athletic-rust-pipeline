use super::archive_io::{
    create_directory, freeze_file, invalid, lock_cache, present, publish_once, replace_cache,
    sync_directory, with_stage, write_file,
};
use super::capture::Capture;
use super::quarantine::{quarantine_pair, Damage};
use super::{
    content_digest, read_cache_file, CacheMeta, FetchError, MAX_BODY_BYTES, MAX_META_BYTES,
};
use std::path::{Path, PathBuf};

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
    with_stage(root, |stage| {
        let lock = match lock_cache(meta_path) {
            Ok(lock) => lock,
            Err(error) => {
                preserve_capture(&archive, stage, "new", body, &capture)?;
                return Err(error);
            }
        };
        preserve_previous(&archive, stage, body_path, meta_path)?;
        let archived_body = preserve_capture(&archive, stage, "new", body, &capture)?;
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
) -> Result<(), FetchError> {
    let root = cache_root(body_path, meta_path)?;
    let capture = Capture::from_meta(meta, meta_path)?;
    capture.verify_body(body, body_path)?;
    let archive = initialize_archive(root)?;
    with_stage(root, |stage| {
        preserve_capture(&archive, stage, "new", body, &capture).map(|_| ())
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
    with_stage(root, |stage| {
        preserve_previous(&archive, stage, body_path, meta_path)
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

fn initialize_archive(root: &Path) -> Result<PathBuf, FetchError> {
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
            preserve_capture(archive, stage, "old", &body, &capture)?;
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

fn preserve_capture(
    archive: &Path,
    stage: &Path,
    label: &str,
    body: &[u8],
    capture: &Capture,
) -> Result<PathBuf, FetchError> {
    let digest = &capture.meta.content_digest;
    let archived_body = archive.join("bodies").join(format!("{digest}.body"));
    if !present(&archived_body)? {
        let staged_body = stage.join(format!("{label}.body"));
        write_file(&staged_body, body)?;
        freeze_file(&staged_body)?;
        publish_once(&staged_body, &archived_body)?;
    }
    if verified_body(&archived_body, capture)? != body {
        return Err(invalid(
            &archived_body,
            "archive body differs from the captured bytes",
        ));
    }
    freeze_file(&archived_body)?;
    sync_directory(&archive.join("bodies"))?;
    let captures = archive.join("captures").join(digest);
    create_directory(&captures)?;
    let metadata_digest = content_digest(&capture.encoded);
    let archived_meta = captures.join(format!("{metadata_digest}.meta.json"));
    if !present(&archived_meta)? {
        let staged_meta = stage.join(format!("{label}.meta.json"));
        write_file(&staged_meta, &capture.encoded)?;
        freeze_file(&staged_meta)?;
        publish_once(&staged_meta, &archived_meta)?;
    }
    verify_metadata(&archived_meta, &capture.encoded)?;
    freeze_file(&archived_meta)?;
    sync_directory(&captures)?;
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
