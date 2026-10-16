use super::*;
use sha2::{Digest, Sha256};

pub(super) fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let mut hasher = Sha256::new();
    hasher.update(b"GET\x1f");
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let full = format!("{:x}", hasher.finalize());
    let key = full.get(..32).ok_or("cache key")?;
    let metadata = json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)), "bytes": body.len(),
        "fetched_at": "2026-10-01T23:44:16Z", "content_type": null,
    });
    std::fs::create_dir_all(fetcher.cache_dir())?;
    std::fs::write(fetcher.cache_dir().join(format!("{key}.body")), body)?;
    std::fs::write(
        fetcher.cache_dir().join(format!("{key}.meta.json")),
        serde_json::to_vec(&metadata)?,
    )?;
    Ok(())
}
