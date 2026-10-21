use super::{sample_archive_artifacts, ArchiveActivityReason, ArchiveActivityScan};
use crate::net::cache::{content_digest, write_cache, CacheMeta, MAX_META_BYTES};
use crate::net::RepresentationHeaders;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct FixtureClock;
impl census_store::clock::Clock for FixtureClock {
    fn today(&self) -> String {
        "2026-10-14".to_string()
    }
    fn today_iso8601(&self) -> String {
        "2026-10-14T00:00:00Z".to_string()
    }
    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

fn meta_for(body: &[u8], url: &str, fetched_at: &str) -> CacheMeta {
    CacheMeta {
        url: url.to_string(),
        observed_http: None,
        response_url: None,
        method: "GET".to_string(),
        representation: RepresentationHeaders::default(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: fetched_at.to_string(),
        etag: None,
        last_modified: None,
        content_type: None,
        redirects: Vec::new(),
    }
}

fn lock_file(root: &Path) -> TestResult {
    std::fs::write(root.join("archive-capacity.lock"), b"")?;
    Ok(())
}

fn sample(root: &Path) -> Result<super::ArchiveActivitySample, ArchiveActivityReason> {
    match sample_archive_artifacts(root, "2026-10-14T00:00:00Z", &FixtureClock) {
        ArchiveActivityScan::Measured(sample) => Ok(sample),
        ArchiveActivityScan::Unmeasured { reason } => Err(reason),
    }
}

#[test]
fn empty_archive_reports_measured_zero() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = root.path().join("archive");
    std::fs::create_dir_all(archive.join("bodies"))?;
    std::fs::create_dir_all(archive.join("captures"))?;
    lock_file(root.path())?;
    let observed = sample(root.path()).map_err(|reason| format!("{reason:?}"))?;
    check!(eq; observed.capture_documents, 0);
    check!(eq; observed.unique_body_files, 0);
    check!(eq; observed.unique_body_payload_bytes, 0);
    Ok(())
}

#[test]
fn distinct_meta_same_body_counts_two_documents_one_body() -> TestResult {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let body = b"same raw body";
    let first = meta_for(body, "https://example.test/a", "2026-09-27T12:00:00Z");
    let mut second = meta_for(body, "https://example.test/b", "2026-09-28T12:00:00Z");
    second.status = 403;
    write_cache(&body_path, &meta_path, body, &first)?;
    write_cache(&body_path, &meta_path, body, &second)?;
    let observed = sample(root.path()).map_err(|reason| format!("{reason:?}"))?;
    check!(eq; observed.capture_documents, 2);
    check!(eq; observed.unique_body_files, 1);
    check!(eq; observed.unique_body_payload_bytes, 13);
    Ok(())
}

#[test]
fn identical_replay_keeps_single_address() -> TestResult {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let body = b"raw";
    let meta = meta_for(body, "https://example.test/source", "2026-09-27T12:00:00Z");
    write_cache(&body_path, &meta_path, body, &meta)?;
    let before = sample(root.path()).map_err(|reason| format!("{reason:?}"))?;
    write_cache(&body_path, &meta_path, body, &meta)?;
    let after = sample(root.path()).map_err(|reason| format!("{reason:?}"))?;
    check!(eq; before.capture_documents, 1);
    check!(eq; after.capture_documents, 1);
    check!(eq; after.unique_body_payload_bytes, 3);
    Ok(())
}

#[test]
fn busy_lock_is_unmeasured_never_zero() -> TestResult {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let body = b"held";
    write_cache(
        &body_path,
        &meta_path,
        body,
        &meta_for(body, "https://example.test/source", "2026-09-27T12:00:00Z"),
    )?;
    let _held = census_store::ArchiveWriteFence::acquire(root.path())?;
    match sample_archive_artifacts(root.path(), "2026-10-14T00:00:00Z", &FixtureClock) {
        ArchiveActivityScan::Unmeasured { reason } => {
            check!(eq; reason, ArchiveActivityReason::FenceBusy);
            Ok(())
        }
        ArchiveActivityScan::Measured(_) => Err("busy scan reported measured".into()),
    }
}

#[test]
fn missing_archive_is_unmeasured() -> TestResult {
    let root = tempfile::tempdir()?;
    match sample_archive_artifacts(root.path(), "2026-10-14T00:00:00Z", &FixtureClock) {
        ArchiveActivityScan::Unmeasured { reason } => {
            check!(eq; reason, ArchiveActivityReason::Missing);
            Ok(())
        }
        ArchiveActivityScan::Measured(_) => Err("missing archive reported measured".into()),
    }
}

#[test]
fn corrupt_meta_is_unmeasured() -> TestResult {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let body = b"good";
    write_cache(
        &body_path,
        &meta_path,
        body,
        &meta_for(body, "https://example.test/source", "2026-09-27T12:00:00Z"),
    )?;
    let archive = root.path().join("archive");
    let mut metas: Vec<std::path::PathBuf> = Vec::new();
    for entry in std::fs::read_dir(archive.join("captures"))? {
        for meta in std::fs::read_dir(entry?.path())? {
            metas.push(meta?.path());
        }
    }
    let target = metas.first().ok_or("missing meta")?.clone();
    std::fs::remove_file(&target)?;
    std::fs::write(&target, b"{not json")?;
    match sample_archive_artifacts(root.path(), "2026-10-14T00:00:00Z", &FixtureClock) {
        ArchiveActivityScan::Unmeasured { reason } => {
            check!(eq; reason, ArchiveActivityReason::Corrupt);
            Ok(())
        }
        ArchiveActivityScan::Measured(_) => Err("corrupt meta reported measured".into()),
    }
}

#[test]
fn oversized_meta_hits_limit() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = root.path().join("archive");
    let content = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let dir = archive.join("captures").join(content);
    std::fs::create_dir_all(archive.join("bodies"))?;
    std::fs::create_dir_all(&dir)?;
    lock_file(root.path())?;
    let oversized = vec![b'x'; MAX_META_BYTES + 1];
    std::fs::write(
        dir.join("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.meta.json"),
        &oversized,
    )?;
    match sample_archive_artifacts(root.path(), "2026-10-14T00:00:00Z", &FixtureClock) {
        ArchiveActivityScan::Unmeasured { reason } => {
            check!(eq; reason, ArchiveActivityReason::Limit);
            Ok(())
        }
        ArchiveActivityScan::Measured(_) => Err("oversized meta reported measured".into()),
    }
}

#[test]
fn symlinked_body_or_fence_is_not_a_measured_inventory() -> TestResult {
    for replace_fence in [false, true] {
        let root = tempfile::tempdir()?;
        let body = b"retained";
        write_cache(
            &root.path().join("source.body"),
            &root.path().join("source.meta.json"),
            body,
            &meta_for(body, "https://example.test/source", "2026-09-27T12:00:00Z"),
        )?;
        let outside = root.path().join("outside");
        std::fs::write(&outside, b"untouched")?;
        let target = if replace_fence {
            root.path().join("archive-capacity.lock")
        } else {
            root.path()
                .join("archive/bodies")
                .join(format!("{}.body", content_digest(body)))
        };
        std::fs::remove_file(&target)?;
        std::os::unix::fs::symlink(&outside, &target)?;
        check!(eq; sample(root.path()), Err(ArchiveActivityReason::UnexpectedLayout));
        check!(eq; std::fs::read(&outside)?, b"untouched");
    }
    Ok(())
}

#[test]
fn total_metadata_budget_is_not_reset_for_each_document() -> TestResult {
    let root = tempfile::tempdir()?;
    let body = b"body";
    let digest = content_digest(body);
    let archive = root.path().join("archive");
    let captures = archive.join("captures").join(&digest);
    std::fs::create_dir_all(&captures)?;
    std::fs::create_dir_all(archive.join("bodies"))?;
    std::fs::write(archive.join("bodies").join(format!("{digest}.body")), body)?;
    lock_file(root.path())?;
    let padding = "a".repeat(60 * 1024);
    let mut written = 0_u64;
    for ordinal in 0..1100 {
        let url = format!("https://example.test/{padding}/{ordinal}");
        let meta = meta_for(body, &url, "2026-09-27T12:00:00Z");
        let capture = crate::net::cache::Capture::from_meta(&meta, &captures)?;
        let name = format!("{}.meta.json", content_digest(&capture.encoded));
        written = written
            .checked_add(u64::try_from(capture.encoded.len())?)
            .ok_or("fixture metadata size overflow")?;
        std::fs::write(captures.join(name), capture.encoded)?;
        if written > super::MAX_ACTIVITY_META_TOTAL_BYTES {
            break;
        }
    }
    check!(written > super::MAX_ACTIVITY_META_TOTAL_BYTES);
    check!(eq; sample(root.path()), Err(ArchiveActivityReason::Limit));
    Ok(())
}
