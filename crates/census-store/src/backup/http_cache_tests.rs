use super::http_cache_integrity;
use sha2::{Digest, Sha256};
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_live(http: &Path, key: &str, body: &[u8], digest: &str) -> TestResult {
    std::fs::create_dir_all(http)?;
    std::fs::write(http.join(format!("{key}.body")), body)?;
    let meta = serde_json::json!({
        "url": "https://example.test/source",
        "method": "GET",
        "status": 200,
        "content_digest": digest,
        "bytes": body.len(),
        "fetched_at": "2026-10-10T00:00:00Z",
    });
    std::fs::write(
        http.join(format!("{key}.meta.json")),
        serde_json::to_vec(&meta)?,
    )?;
    Ok(())
}

fn write_archived_body(http: &Path, body: &[u8]) -> TestResult {
    let bodies = http.join("archive/bodies");
    std::fs::create_dir_all(&bodies)?;
    std::fs::write(bodies.join(format!("{}.body", hex(body))), body)?;
    Ok(())
}

fn write_archived_capture(http: &Path, content_digest: &str, metadata: &[u8]) -> TestResult {
    let directory = http.join("archive/captures").join(content_digest);
    std::fs::create_dir_all(&directory)?;
    std::fs::write(
        directory.join(format!("{}.meta.json", hex(metadata))),
        metadata,
    )?;
    Ok(())
}

fn archived_meta(content_digest: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(serde_json::to_vec(&serde_json::json!({
        "url": "https://example.test/source",
        "method": "GET",
        "status": 200,
        "content_digest": content_digest,
        "bytes": 4,
        "fetched_at": "2026-10-10T00:00:00Z",
    }))?)
}

#[test]
fn a_healthy_cache_reports_no_issues() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    let body = b"captured bytes";
    write_live(&http, "livekey", body, &hex(body))?;
    write_archived_body(&http, body)?;
    let meta = archived_meta(&hex(body))?;
    write_archived_capture(&http, &hex(body), &meta)?;
    std::fs::create_dir_all(http.join("quarantine"))?;
    std::fs::write(http.join("livekey.meta.json.lock"), b"")?;
    let issues = http_cache_integrity(root.path())?;
    check!(issues.is_empty(), "healthy cache produced: {issues:?}");
    Ok(())
}

#[test]
fn a_live_meta_that_does_not_describe_its_body_is_reported() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    let other = b"different bytes";
    write_live(&http, "livekey", b"captured bytes", &hex(other))?;
    let issues = http_cache_integrity(root.path())?;
    check!(
        issues.iter().any(|issue| issue.contains("digest mismatch")),
        "mismatched live digest produced: {issues:?}"
    );
    Ok(())
}

#[test]
fn an_orphaned_live_body_is_reported() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    std::fs::create_dir_all(&http)?;
    std::fs::write(http.join("orphan.body"), b"orphan")?;
    let issues = http_cache_integrity(root.path())?;
    check!(
        issues.iter().any(|issue| issue.contains("has no metadata")),
        "orphaned live body produced: {issues:?}"
    );
    Ok(())
}

#[test]
fn an_archive_body_name_must_match_its_bytes() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    let bodies = http.join("archive/bodies");
    std::fs::create_dir_all(&bodies)?;
    std::fs::write(bodies.join(format!("{}.body", hex(b"claimed"))), b"actual")?;
    let issues = http_cache_integrity(root.path())?;
    check!(
        issues.iter().any(|issue| issue.contains("digest mismatch")),
        "archive body mismatch produced: {issues:?}"
    );
    Ok(())
}

#[test]
fn an_archive_body_without_a_capture_is_reported() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    write_archived_body(&http, b"captured bytes")?;
    let issues = http_cache_integrity(root.path())?;
    check!(
        issues
            .iter()
            .any(|issue| issue.contains("has no capture metadata")),
        "orphaned archive body produced: {issues:?}"
    );
    Ok(())
}

#[test]
fn an_archive_capture_without_metadata_is_reported() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    let digest = hex(b"captured bytes");
    write_archived_body(&http, b"captured bytes")?;
    std::fs::create_dir_all(http.join("archive/captures").join(&digest))?;
    let issues = http_cache_integrity(root.path())?;
    check!(
        issues
            .iter()
            .any(|issue| issue.contains("capture metadata is missing")),
        "empty capture directory produced: {issues:?}"
    );
    Ok(())
}

#[test]
fn an_archive_capture_referencing_a_missing_body_is_reported() -> TestResult {
    let root = tempfile::tempdir()?;
    let http = root.path().join("http");
    let digest = hex(b"captured bytes");
    let meta = archived_meta(&digest)?;
    write_archived_capture(&http, &digest, &meta)?;
    let issues = http_cache_integrity(root.path())?;
    check!(
        issues
            .iter()
            .any(|issue| issue.contains("referenced body is missing")),
        "missing archive body produced: {issues:?}"
    );
    Ok(())
}
