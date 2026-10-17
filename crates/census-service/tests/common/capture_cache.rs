use std::path::Path;

use anyhow::{Context, Result};
use census_crawl::net::RepresentationHeaders;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Default)]
pub(super) enum ResponseUrl<'a> {
    #[default]
    Absent,
    Null,
    Url(&'a str),
}

#[derive(Default)]
pub(super) struct Metadata<'a> {
    pub headers: Option<&'a [(String, String)]>,
    pub content_type: Option<&'a str>,
    pub response_url: ResponseUrl<'a>,
}

pub(super) fn seed(
    cache: &Path,
    url: &str,
    body: &[u8],
    acquired_at: &str,
    headers: &[(String, String)],
) -> Result<()> {
    seed_response(cache, url, body, acquired_at, Metadata {
        headers: Some(headers),
        ..Metadata::default()
    })
}

pub(super) fn seed_response(
    cache: &Path,
    url: &str,
    body: &[u8],
    acquired_at: &str,
    metadata: Metadata<'_>,
) -> Result<()> {
    let representation = metadata.headers.map(RepresentationHeaders::canonical).transpose()?;
    let mut key = Sha256::new();
    key.update(b"GET");
    key.update([0x1f]);
    key.update(url.as_bytes());
    key.update([0x1f]);
    if let Some(representation) = &representation {
        key.update(representation.identity().as_bytes());
    }
    let digest = key.finalize();
    let prefix = digest.get(..16).context("SHA-256 prefix missing")?;
    let key: String = prefix.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    let mut meta = json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)),
        "bytes": body.len(), "fetched_at": acquired_at,
    });
    if let Some(representation) = representation {
        meta["representation"] = serde_json::to_value(representation)?;
    }
    if let Some(content_type) = metadata.content_type {
        meta["content_type"] = json!(content_type);
    }
    match metadata.response_url {
        ResponseUrl::Absent => {}
        ResponseUrl::Null => meta["response_url"] = serde_json::Value::Null,
        ResponseUrl::Url(url) => meta["response_url"] = json!(url),
    }
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    Ok(())
}

#[path = "capture_cache_tests.rs"]
mod tests;
