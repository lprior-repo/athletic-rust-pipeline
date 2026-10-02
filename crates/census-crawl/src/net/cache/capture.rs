use super::archive_io::invalid;
use super::{
    content_digest, is_valid_hex64, CacheMeta, FetchError, MAX_BODY_BYTES, MAX_META_BYTES,
};
use serde::Deserialize;
use std::io::{self, Write};
use std::path::Path;

pub(super) struct Capture {
    pub(super) meta: CacheMeta,
    pub(super) encoded: Vec<u8>,
}

impl Capture {
    pub(super) fn from_meta(meta: &CacheMeta, path: &Path) -> Result<Self, FetchError> {
        let fields = [
            meta.url.as_str(),
            meta.method.as_str(),
            meta.content_digest.as_str(),
            meta.fetched_at.as_str(),
            meta.etag.as_deref().map_or("", |value| value),
            meta.last_modified.as_deref().map_or("", |value| value),
            meta.content_type.as_deref().map_or("", |value| value),
        ];
        let length = fields
            .iter()
            .try_fold(0_usize, |total, value| total.checked_add(value.len()));
        if length.is_none_or(|length| length > MAX_META_BYTES) {
            return Err(invalid(path, "capture metadata exceeds its limit"));
        }
        let mut value = serde_json::to_value(meta).map_err(|source| FetchError::Encode {
            target: path.display().to_string(),
            source,
        })?;
        value.sort_all_objects();
        Self::from_value(&value, path)
    }

    pub(super) fn from_bytes(bytes: &[u8], path: &Path) -> Result<Self, FetchError> {
        let mut value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|source| FetchError::Decode {
                target: path.display().to_string(),
                source,
            })?;
        value.sort_all_objects();
        Self::from_value(&value, path)
    }

    fn from_value(value: &serde_json::Value, path: &Path) -> Result<Self, FetchError> {
        let meta = CacheMeta::deserialize(value).map_err(|source| FetchError::Decode {
            target: path.display().to_string(),
            source,
        })?;
        if !value.is_object()
            || !is_valid_hex64(&meta.content_digest)
            || meta.bytes > MAX_BODY_BYTES
        {
            return Err(invalid(
                path,
                "capture metadata declares invalid body integrity",
            ));
        }
        let mut writer = BoundedMetadata(Vec::new());
        serde_json::to_writer_pretty(&mut writer, value).map_err(|source| FetchError::Encode {
            target: path.display().to_string(),
            source,
        })?;
        Ok(Self {
            meta,
            encoded: writer.0,
        })
    }

    pub(super) fn verify_body(&self, body: &[u8], path: &Path) -> Result<(), FetchError> {
        if body.len() != self.meta.bytes || content_digest(body) != self.meta.content_digest {
            return Err(invalid(
                path,
                "capture body does not match its declared length and SHA256",
            ));
        }
        Ok(())
    }
}

struct BoundedMetadata(Vec<u8>);

impl Write for BoundedMetadata {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let length = self
            .0
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("metadata length overflow"))?;
        if length > MAX_META_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "capture metadata exceeds its limit",
            ));
        }
        self.0.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
