use super::{FetchError, Fetcher, MAX_BODY_BYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_META_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct CacheMeta {
    pub(crate) url: String,
    pub(crate) method: String,
    pub(crate) status: u16,
    pub(crate) content_digest: String,
    pub(crate) bytes: usize,
    pub(crate) fetched_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) content_type: Option<String>,
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

fn read_cache_file(
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
    let mut body = Vec::with_capacity(size);
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
) -> Result<Option<(CacheMeta, Vec<u8>)>, FetchError> {
    if !meta_path.exists() || !body_path.exists() {
        return Ok(None);
    }
    let meta_bytes =
        read_cache_file(meta_path, MAX_META_BYTES, None)?.ok_or_else(|| FetchError::Cache {
            path: meta_path.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "cache metadata exceeds its limit or changed during reading",
            ),
        })?;
    let meta: CacheMeta =
        serde_json::from_slice(&meta_bytes).map_err(|source| FetchError::Decode {
            target: meta_path.display().to_string(),
            source,
        })?;
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

pub(crate) fn write_cache(
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

#[cfg(test)]
mod tests;
