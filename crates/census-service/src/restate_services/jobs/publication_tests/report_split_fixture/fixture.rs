use super::super::TestResult;
use census_crawl::net::FetchOutcome;
use census_store::Store;
use serde_json::json;
use sha2::{Digest, Sha256};

pub(super) fn retain(store: &Store, capture: &FetchOutcome) -> TestResult {
    let root = store.http_cache_dir().join("archive");
    let captures = root.join("captures").join(&capture.content_digest);
    std::fs::create_dir_all(root.join("bodies"))?;
    std::fs::create_dir_all(&captures)?;
    std::fs::write(
        root.join("bodies")
            .join(format!("{}.body", capture.content_digest)),
        &capture.body,
    )?;
    let metadata = serde_json::to_vec(&json!({
        "url": capture.url, "response_url": null, "method": "GET", "status": 200,
        "content_digest": capture.content_digest, "bytes": capture.bytes,
        "fetched_at": capture.fetched_at, "content_type": "text/html", "etag": "synthetic fixture", "redirects": [],
    }))?;
    std::fs::write(
        captures.join(format!("{}.meta.json", hash(&metadata))),
        metadata,
    )?;
    Ok(())
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
