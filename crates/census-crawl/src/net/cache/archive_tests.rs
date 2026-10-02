use super::*;
use std::io::ErrorKind;

fn metadata(body: &[u8]) -> CacheMeta {
    CacheMeta {
        url: "https://example.test/source".to_owned(),
        method: "GET".to_owned(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-09-27T12:34:56Z".to_owned(),
        etag: Some("source-version".to_owned()),
        last_modified: Some("Sat, 26 Sep 2026 10:00:00 GMT".to_owned()),
        content_type: Some("application/json".to_owned()),
    }
}

fn cache_paths(root: &Path) -> (PathBuf, PathBuf) {
    (root.join("source.body"), root.join("source.meta.json"))
}

fn archive_body(root: &Path, digest: &str) -> PathBuf {
    root.join("archive/bodies").join(format!("{digest}.body"))
}

fn archive_meta(root: &Path, meta: &CacheMeta) -> PathBuf {
    let capture = capture::Capture::from_meta(meta, root).expect("capture identity");
    root.join("archive/captures")
        .join(&meta.content_digest)
        .join(format!("{}.meta.json", content_digest(&capture.encoded)))
}

fn assert_capture(root: &Path, body: &[u8], meta: &CacheMeta) {
    let saved_body =
        std::fs::read(archive_body(root, &meta.content_digest)).expect("archived body");
    assert_eq!(saved_body, body);
    assert_eq!(content_digest(&saved_body), meta.content_digest);
    let encoded = std::fs::read(archive_meta(root, meta)).expect("archived metadata");
    let actual: serde_json::Value = serde_json::from_slice(&encoded).expect("capture metadata");
    assert_eq!(
        actual,
        serde_json::to_value(meta).expect("expected metadata")
    );
}

fn file_names(path: &Path) -> Vec<std::ffi::OsString> {
    let mut names: Vec<_> = std::fs::read_dir(path)
        .expect("archive directory")
        .map(|entry| entry.expect("archive entry").file_name())
        .collect();
    names.sort();
    names
}

fn quarantine_bundles(root: &Path) -> Vec<PathBuf> {
    let quarantine = root.join("quarantine");
    file_names(&quarantine)
        .into_iter()
        .map(|name| quarantine.join(name))
        .collect()
}

fn assert_quarantined(
    root: &Path,
    body: Option<&[u8]>,
    meta: Option<&[u8]>,
    reason: &str,
) -> PathBuf {
    let bundles = quarantine_bundles(root);
    assert_eq!(bundles.len(), 1);
    let evidence = bundles.into_iter().next().expect("quarantine bundle");
    let (body_path, meta_path) = cache_paths(root);
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(evidence.join("reason.json")).expect("durable reason"),
    )
    .expect("reason");
    assert_eq!(
        record,
        serde_json::json!({
            "version": 1,
            "body_path": body_path,
            "meta_path": meta_path,
            "body_present": body.is_some(),
            "meta_present": meta.is_some(),
            "reason": reason,
        })
    );
    for (name, expected) in [("body", body), ("meta.json", meta)] {
        match expected {
            Some(expected) => assert_eq!(
                std::fs::read(evidence.join(name)).expect("evidence"),
                expected
            ),
            None => assert!(!evidence.join(name).exists()),
        }
    }
    evidence
}

fn assert_served(root: &Path, body: &[u8], meta: &CacheMeta) {
    let (body_path, meta_path) = cache_paths(root);
    let (served, actual) = read_cache(&body_path, &meta_path)
        .expect("cache read")
        .expect("hit");
    assert_eq!(actual, body);
    assert_eq!(
        serde_json::to_value(served).expect("served metadata"),
        serde_json::to_value(meta).expect("expected")
    );
}

#[test]
fn refresh_retains_both_raw_captures_and_serves_the_replacement() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"raw A");
    let mut second = metadata(b"raw B");
    second.fetched_at = "2026-09-28T12:34:56Z".to_owned();
    write_cache(&body_path, &meta_path, b"raw A", &first).expect("first capture");
    write_cache(&body_path, &meta_path, b"raw B", &second).expect("refresh");
    assert_capture(root.path(), b"raw A", &first);
    assert_capture(root.path(), b"raw B", &second);
    let (served, body) = read_cache(&body_path, &meta_path)
        .expect("read")
        .expect("hit");
    assert_eq!(body, b"raw B");
    assert_eq!(served.fetched_at, second.fetched_at);
}

#[test]
fn identical_replay_preserves_the_original_audit_files_without_duplicates() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    write_cache(&body_path, &meta_path, b"raw", &meta).expect("first capture");
    let archive_path = archive_meta(root.path(), &meta);
    let before = std::fs::read(&archive_path).expect("original metadata");
    let modified = std::fs::metadata(&archive_path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    assert_eq!(
        replay_cache(&body_path, &meta_path, &meta).expect("replay"),
        Some(b"raw".to_vec())
    );
    assert_eq!(
        std::fs::read(&archive_path).expect("replayed metadata"),
        before
    );
    assert_eq!(
        std::fs::metadata(&archive_path)
            .expect("metadata")
            .modified()
            .expect("mtime"),
        modified
    );
    assert_eq!(
        file_names(archive_path.parent().expect("capture directory")),
        vec![archive_path.file_name().expect("name").to_owned()]
    );
    assert_eq!(
        file_names(&root.path().join("archive/bodies")),
        vec![archive_body(root.path(), &meta.content_digest)
            .file_name()
            .expect("name")
            .to_owned()]
    );
}

#[test]
fn identical_bodies_retain_distinct_source_method_status_and_capture_time() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"same raw body");
    let mut second = first.clone();
    second.url = "https://example.test/other-source".to_owned();
    second.method = "POST".to_owned();
    second.status = 403;
    second.fetched_at = "2026-09-28T12:34:56Z".to_owned();
    write_cache(&body_path, &meta_path, b"same raw body", &first).expect("first capture");
    write_cache(&body_path, &meta_path, b"same raw body", &second).expect("second capture");
    assert_capture(root.path(), b"same raw body", &first);
    assert_capture(root.path(), b"same raw body", &second);
    let expected = vec![
        archive_meta(root.path(), &first),
        archive_meta(root.path(), &second),
    ];
    assert_ne!(expected[0], expected[1]);
    assert_eq!(
        file_names(&root.path().join("archive/bodies")),
        vec![archive_body(root.path(), &first.content_digest)
            .file_name()
            .expect("name")
            .to_owned()]
    );
    assert!(read_cache(&body_path, &meta_path)
        .expect("non-success remains ineligible")
        .is_none());
}

#[test]
fn migration_preserves_verified_old_non_success_capture_and_unknown_metadata() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let mut old = metadata(b"source refusal");
    old.status = 503;
    let mut full = serde_json::to_value(&old).expect("old metadata");
    full["provider_capture"] = serde_json::json!({"request_id": "retained-request"});
    let encoded = serde_json::to_vec(&full).expect("old format");
    std::fs::write(&body_path, b"source refusal").expect("old body");
    std::fs::write(&meta_path, &encoded).expect("old metadata");
    let replacement = metadata(b"new success");
    write_cache(&body_path, &meta_path, b"new success", &replacement).expect("migration");
    let captured = capture::Capture::from_bytes(&encoded, &meta_path).expect("full capture");
    let path = root
        .path()
        .join("archive/captures")
        .join(&old.content_digest)
        .join(format!("{}.meta.json", content_digest(&captured.encoded)));
    let retained: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("retained metadata")).expect("metadata");
    assert_eq!(retained, full);
    assert_eq!(
        std::fs::read(archive_body(root.path(), &old.content_digest)).expect("old raw"),
        b"source refusal"
    );
    assert_capture(root.path(), b"new success", &replacement);
}

#[test]
fn migrating_an_identical_compact_capture_does_not_manufacture_an_audit_record() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    std::fs::write(&body_path, b"raw").expect("legacy body");
    std::fs::write(
        &meta_path,
        serde_json::to_vec(&meta).expect("compact metadata"),
    )
    .expect("legacy metadata");
    write_cache(&body_path, &meta_path, b"raw", &meta).expect("migration replay");
    assert_capture(root.path(), b"raw", &meta);
    let path = archive_meta(root.path(), &meta);
    assert_eq!(
        file_names(path.parent().expect("captures")),
        vec![path.file_name().expect("name").to_owned()]
    );
}

mod captured_smoke;
mod failures;
mod quarantine_crashes;
mod races;
mod recovery;
