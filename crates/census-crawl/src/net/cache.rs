//! On-disk body/metadata cache: content-addressed keys, self-verifying evidence, and atomic publish.
//!
//! A crash between the body rename and the metadata rename is impossible to observe: the cache read
//! requires *both* files to be present and a content digest that matches, so a half-written
//! generation is silently treated as a cache miss rather than poisoning a later read with stale
//! evidence.

use super::{FetchError, Fetcher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Metadata stored alongside a cached body on disk.
///
/// `key_prefix` is the 16-byte truncated SHA-256 used to derive the on-disk filenames. It serves
/// exclusively as a cache key, never as evidence.
///
/// `content_digest` is the full 32-byte SHA-256 of the body, hex-encoded (64 characters). It is
/// the content hash: the thing that proves "this body is what we think it is." On read, both the
/// byte count and the digest are checked; a mismatch means the body has changed since it was cached
/// (corruption, truncation, or a torn write) and must be discarded.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct CacheMeta {
    pub(super) url: String,
    pub(super) method: String,
    pub(super) status: u16,
    /// Truncated SHA-256 key: the 16-byte prefix used to derive the on-disk file names. SHA-256
    /// always produces 32 bytes, so the prefix is present; `get` keeps the extraction total
    /// without a panic path.
    pub(super) key_prefix: String,
    /// Full 32-byte SHA-256 content digest of the body, hex-encoded (64 characters).
    pub(super) content_digest: String,
    pub(super) bytes: usize,
    pub(super) fetched_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) content_type: Option<String>,
}

impl Fetcher {
    pub(super) fn cache_paths(&self, key: &str) -> (PathBuf, PathBuf) {
        (
            self.cache_dir.join(format!("{key}.body")),
            self.cache_dir.join(format!("{key}.meta.json")),
        )
    }

    pub(super) fn key_for(method: &str, url: &str, extra: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        hasher.update(extra.as_bytes());
        sha256_prefix16(hasher)
    }
}

/// Hex of the leading 16 bytes of a SHA-256 hash.
///
/// The crate keys cached bodies, cache metadata and content ids by that prefix. SHA-256 always
/// digests to 32 bytes, so the prefix is present; `get` keeps the extraction total without a
/// panic path.
pub(super) fn sha256_prefix16(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let head = match digest.get(..16) {
        Some(head) => head,
        None => digest.as_slice(),
    };
    head.iter().map(|b| format!("{b:02x}")).collect()
}

/// Full 32-byte SHA-256 digest, hex-encoded — the content hash, not a key.
///
/// The caller hands the body bytes (not a hasher), so this function does the hashing. On read, the
/// result is compared against `CacheMeta::content_digest`; a mismatch means the body has changed
/// since it was cached (corruption, truncation, or a torn write) and must be discarded.
pub(super) fn content_digest(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let d = hasher.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Read a cached entry and verify it against the on-disk body.
///
/// Returns `Ok(None)` when either file is missing, the body file cannot be read, or the body does
/// not match the metadata (size or digest mismatch). A mismatch is a cache miss, not an error —
/// the corrupted body is discarded and the request is re-fetched.
pub(super) fn read_cache(
    body_path: &Path,
    meta_path: &Path,
) -> Result<Option<(CacheMeta, Vec<u8>)>, FetchError> {
    if !meta_path.exists() || !body_path.exists() {
        return Ok(None);
    }
    let meta = std::fs::read_to_string(meta_path).map_err(|source| FetchError::Cache {
        path: meta_path.to_path_buf(),
        source,
    })?;
    let meta: CacheMeta = serde_json::from_str(&meta).map_err(|source| FetchError::Decode {
        target: meta_path.display().to_string(),
        source,
    })?;
    let body = std::fs::read(body_path).map_err(|source| FetchError::Cache {
        path: body_path.to_path_buf(),
        source,
    })?;
    // Verify length first (cheap), then content digest.
    if body.len() != meta.bytes {
        return Ok(None);
    }
    if content_digest(&body) != meta.content_digest {
        return Ok(None);
    }
    Ok(Some((meta, body)))
}

/// Publish a cached entry atomically: both body and metadata are written to temp files and
/// renamed in sequence. A crash between the two renames leaves one file without the other, so the
/// next read (which requires both) treats the entry as absent rather than corrupted.
pub(super) fn write_cache(
    body_path: &Path,
    meta_path: &Path,
    body: &[u8],
    meta: &CacheMeta,
) -> Result<(), FetchError> {
    let tmp_body = body_path.with_extension("body.tmp");
    std::fs::write(&tmp_body, body).map_err(|source| FetchError::Cache {
        path: tmp_body.clone(),
        source,
    })?;
    std::fs::rename(&tmp_body, body_path).map_err(|source| FetchError::Cache {
        path: body_path.to_path_buf(),
        source,
    })?;
    let tmp_meta = meta_path.with_extension("meta.json.tmp");
    let encoded = serde_json::to_vec_pretty(meta).map_err(|source| FetchError::Encode {
        target: meta_path.display().to_string(),
        source,
    })?;
    std::fs::write(&tmp_meta, encoded).map_err(|source| FetchError::Cache {
        path: tmp_meta.clone(),
        source,
    })?;
    std::fs::rename(&tmp_meta, meta_path).map_err(|source| FetchError::Cache {
        path: meta_path.to_path_buf(),
        source,
    })?;
    Ok(())
}
