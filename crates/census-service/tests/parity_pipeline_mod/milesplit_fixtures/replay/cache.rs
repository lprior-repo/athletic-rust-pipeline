use std::path::Path;

use anyhow::{Context, Result};
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) fn seed(cache: &Path, url: &str, body: &[u8], acquired_at: &str) -> Result<()> {
    let mut key = Sha256::new();
    key.update(b"GET");
    key.update([0x1f]);
    key.update(url.as_bytes());
    key.update([0x1f]);
    let digest = key.finalize();
    let prefix = digest.get(..16).context("SHA-256 prefix missing")?;
    let key: String = prefix.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    let meta = json!({
        "url": url, "method": "GET", "status": 200,
        "content_digest": format!("{:x}", Sha256::digest(body)),
        "bytes": body.len(), "fetched_at": acquired_at,
    });
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    Ok(())
}
