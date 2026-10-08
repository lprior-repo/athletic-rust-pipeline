use super::{FetchError, Fetcher, RepresentationHeaders, MAX_BODY_BYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

mod archive;
mod archive_io;
mod capture;
mod quarantine;

const MAX_META_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CacheMeta {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) response_url: Option<String>,
    pub(crate) method: String,
    #[serde(default, skip_serializing_if = "RepresentationHeaders::is_empty")]
    pub(crate) representation: RepresentationHeaders,
    pub(crate) status: u16,
    pub(crate) content_digest: String,
    pub(crate) bytes: usize,
    pub(crate) fetched_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

impl Fetcher {
    pub(crate) fn cache_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.cache_dir.join(format!("{key}.body")),
            self.cache_dir.join(format!("{key}.meta.json")),
        )
    }

    pub(crate) fn key_for(method: &str, url: &str, extra: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        hasher.update(extra.as_bytes());
        sha256_prefix16(hasher)
    }
}

pub(crate) fn sha256_prefix16(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let head = match digest.get(..16) {
        Some(head) => head,
        None => digest.as_slice(),
    };
    head.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn content_digest(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let d = hasher.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn is_valid_hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn read_cache_file(
    path: &Path,
    limit: usize,
    expected: Option<usize>,
) -> Result<Option<Vec<u8>>, FetchError> {
    let read = || -> std::io::Result<Option<Vec<u8>>> {
        let mut file = File::open(path)?;
        let size = usize::try_from(file.metadata()?.len())
            .map_err(|_| std::io::Error::other("cache file length cannot be represented"))?;
        if size > limit || expected.is_some_and(|expected| expected != size) {
            return Ok(None);
        }
        read_snapshot(&mut file, size)
    };
    read().map_err(|source| FetchError::Cache {
        path: path.to_path_buf(),
        source,
    })
}

fn read_snapshot(reader: &mut impl Read, size: usize) -> std::io::Result<Option<Vec<u8>>> {
    let limit = u64::try_from(size)
        .map_err(|_| std::io::Error::other("cache read length cannot be represented"))?;
    let mut body = Vec::new();
    body.try_reserve_exact(size)
        .map_err(std::io::Error::other)?;
    (&mut *reader).take(limit).read_to_end(&mut body)?;
    if body.len() != size {
        return Ok(None);
    }
    match reader.read_exact(&mut [0_u8; 1]) {
        Ok(()) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(Some(body)),
        Err(error) => Err(error),
    }
}

pub(crate) fn read_cache(
    body_path: &Path,
    meta_path: &Path,
    method: &str,
    url: &str,
    representation: &RepresentationHeaders,
) -> Result<Option<(CacheMeta, Vec<u8>)>, FetchError> {
    if !meta_path.exists() || !body_path.exists() {
        return Ok(None);
    }
    let Some(meta_bytes) = read_cache_file(meta_path, MAX_META_BYTES, None)? else {
        return Ok(None);
    };
    let meta: CacheMeta = match serde_json::from_slice(&meta_bytes) {
        Ok(meta) => meta,
        Err(_) => return Ok(None),
    };
    if meta.method != method || meta.url != url || &meta.representation != representation {
        return Ok(None);
    }
    if meta.status != 200 || !is_valid_hex64(&meta.content_digest) || meta.bytes > MAX_BODY_BYTES {
        return Ok(None);
    }
    let Some(body) = read_cache_file(body_path, MAX_BODY_BYTES, Some(meta.bytes))? else {
        return Ok(None);
    };
    if content_digest(&body) != meta.content_digest {
        return Ok(None);
    }
    Ok(Some((meta, body)))
}

pub(crate) fn replay_cache(
    body_path: &Path,
    meta_path: &Path,
    expected: &CacheMeta,
) -> Result<Option<Vec<u8>>, FetchError> {
    archive::replay_preserved_cache(body_path, meta_path, expected)
}

pub(crate) fn write_cache(
    body_path: &Path,
    meta_path: &Path,
    body: &[u8],
    meta: &CacheMeta,
) -> Result<(), FetchError> {
    archive::write_preserved_cache(body_path, meta_path, body, meta)
}

pub(crate) fn write_archive(
    body_path: &Path,
    meta_path: &Path,
    body: &[u8],
    meta: &CacheMeta,
) -> Result<PathBuf, FetchError> {
    archive::write_preserved_capture(body_path, meta_path, body, meta)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod archive_tests;
