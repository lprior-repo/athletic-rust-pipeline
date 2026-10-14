use crate::net::cache::CacheMeta;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

pub(super) struct Metadata<'a>(pub(super) &'a CacheMeta);

impl Serialize for Metadata<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let meta = self.0;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("bytes", &meta.bytes)?;
        map.serialize_entry("content_digest", &meta.content_digest)?;
        if let Some(value) = &meta.content_type {
            map.serialize_entry("content_type", value)?;
        }
        if let Some(value) = &meta.etag {
            map.serialize_entry("etag", value)?;
        }
        map.serialize_entry("fetched_at", &meta.fetched_at)?;
        if let Some(value) = &meta.last_modified {
            map.serialize_entry("last_modified", value)?;
        }
        map.serialize_entry("method", &meta.method)?;
        if !meta.redirects.is_empty() {
            map.serialize_entry("redirects", &meta.redirects)?;
        }
        if !meta.representation.is_empty() {
            map.serialize_entry("representation", &meta.representation)?;
        }
        if let Some(value) = &meta.response_url {
            map.serialize_entry("response_url", value)?;
        }
        map.serialize_entry("status", &meta.status)?;
        map.serialize_entry("url", &meta.url)?;
        map.end()
    }
}
