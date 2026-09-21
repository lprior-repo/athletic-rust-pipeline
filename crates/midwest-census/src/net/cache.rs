//! On-disk body/metadata cache: content-addressed keys and atomic writes.

use super::{FetchError, Fetcher};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Metadata stored alongside a cached body on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct CacheMeta {
    pub(super) url: String,
    pub(super) method: String,
    pub(super) status: u16,
    pub(super) sha256: String,
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

pub(super) fn read_cache(
    body_path: &Path,
    meta_path: &Path,
) -> Result<Option<CacheMeta>, FetchError> {
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
    Ok(Some(meta))
}

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
