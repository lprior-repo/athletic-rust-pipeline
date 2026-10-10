use super::*;
use std::io::ErrorKind;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn metadata(body: &[u8]) -> CacheMeta {
    CacheMeta {
        redirects: Vec::new(),
        representation: RepresentationHeaders::default(),
        url: "https://example.test/source".to_owned(),
        response_url: None,
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

fn archive_meta(root: &Path, meta: &CacheMeta) -> TestResult<PathBuf> {
    let capture = capture::Capture::from_meta(meta, root)?;
    Ok(root
        .join("archive/captures")
        .join(&meta.content_digest)
        .join(format!("{}.meta.json", content_digest(&capture.encoded))))
}

fn assert_capture(root: &Path, body: &[u8], meta: &CacheMeta) -> TestResult {
    let saved_body = std::fs::read(archive_body(root, &meta.content_digest))?;
    check!(eq; saved_body, body);
    check!(eq; content_digest(&saved_body), meta.content_digest);
    let encoded = std::fs::read(archive_meta(root, meta)?)?;
    let actual: serde_json::Value = serde_json::from_slice(&encoded)?;
    check!(eq;
        actual,
        serde_json::to_value(meta)?
    );
    Ok(())
}

fn file_names(path: &Path) -> TestResult<Vec<std::ffi::OsString>> {
    let mut names: Vec<_> = std::fs::read_dir(path)?
        .map(|entry| Ok(entry?.file_name()))
        .collect::<TestResult<_>>()?;
    names.sort();
    Ok(names)
}

fn quarantine_bundles(root: &Path) -> TestResult<Vec<PathBuf>> {
    let quarantine = root.join("quarantine");
    Ok(file_names(&quarantine)?
        .into_iter()
        .map(|name| quarantine.join(name))
        .collect())
}

fn assert_quarantined(
    root: &Path,
    body: Option<&[u8]>,
    meta: Option<&[u8]>,
    reason: &str,
) -> TestResult<PathBuf> {
    let bundles = quarantine_bundles(root)?;
    check!(eq; bundles.len(), 1);
    let evidence = bundles.into_iter().next().ok_or("quarantine bundle")?;
    let (body_path, meta_path) = cache_paths(root);
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(evidence.join("reason.json"))?)?;
    check!(eq;
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
            Some(expected) => {
                check!(eq; std::fs::read(evidence.join(name))?, expected);
            }
            None => {
                check!(!evidence.join(name).exists());
            }
        }
    }
    Ok(evidence)
}

fn assert_served(root: &Path, body: &[u8], meta: &CacheMeta) -> TestResult {
    let (body_path, meta_path) = cache_paths(root);
    let (served, actual) = read_cache(
        &body_path,
        &meta_path,
        &meta.method,
        &meta.url,
        &meta.representation,
    )?
    .ok_or("hit")?;
    check!(eq; actual, body);
    check!(eq;
        serde_json::to_value(served)?,
        serde_json::to_value(meta)?
    );
    Ok(())
}

#[test]
fn refresh_retains_both_raw_captures_and_serves_the_replacement() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"raw A");
    let mut second = metadata(b"raw B");
    second.fetched_at = "2026-09-28T12:34:56Z".to_owned();
    write_cache(&body_path, &meta_path, b"raw A", &first)?;
    write_cache(&body_path, &meta_path, b"raw B", &second)?;
    assert_capture(root.path(), b"raw A", &first)?;
    assert_capture(root.path(), b"raw B", &second)?;
    let (served, body) = read_cache(
        &body_path,
        &meta_path,
        &second.method,
        &second.url,
        &second.representation,
    )?
    .ok_or("hit")?;
    check!(eq; body, b"raw B");
    check!(eq; served.fetched_at, second.fetched_at);
    Ok(())
}

#[test]
fn identical_replay_preserves_the_original_audit_files_without_duplicates() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    write_cache(&body_path, &meta_path, b"raw", &meta)?;
    let archive_path = archive_meta(root.path(), &meta)?;
    let before = std::fs::read(&archive_path)?;
    let modified = std::fs::metadata(&archive_path)?.modified()?;
    check!(eq;
        replay_cache(&body_path, &meta_path, &meta)?,
        Some(b"raw".to_vec())
    );
    check!(eq;
        std::fs::read(&archive_path)?,
        before
    );
    check!(eq;
        std::fs::metadata(&archive_path)?.modified()?,
        modified
    );
    check!(eq;
        file_names(archive_path.parent().ok_or("capture directory")?)?,
        vec![archive_path.file_name().ok_or("name")?.to_owned()]
    );
    check!(eq;
        file_names(&root.path().join("archive/bodies"))?,
        vec![archive_body(root.path(), &meta.content_digest)
            .file_name()
            .ok_or("name")?
            .to_owned()]
    );
    Ok(())
}

#[test]
fn identical_bodies_retain_distinct_source_method_status_and_capture_time() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"same raw body");
    let mut second = first.clone();
    second.url = "https://example.test/other-source".to_owned();
    second.method = "POST".to_owned();
    second.status = 403;
    second.fetched_at = "2026-09-28T12:34:56Z".to_owned();
    write_cache(&body_path, &meta_path, b"same raw body", &first)?;
    write_cache(&body_path, &meta_path, b"same raw body", &second)?;
    assert_capture(root.path(), b"same raw body", &first)?;
    assert_capture(root.path(), b"same raw body", &second)?;
    let expected = [
        archive_meta(root.path(), &first)?,
        archive_meta(root.path(), &second)?,
    ];
    check!(ne; expected[0], expected[1]);
    check!(eq;
        file_names(&root.path().join("archive/bodies"))?,
        vec![archive_body(root.path(), &first.content_digest)
            .file_name()
            .ok_or("name")?
            .to_owned()]
    );
    check!(read_cache(
        &body_path,
        &meta_path,
        &second.method,
        &second.url,
        &second.representation
    )?
    .is_none());
    Ok(())
}

#[test]
fn migration_preserves_verified_old_non_success_capture_and_unknown_metadata() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let mut old = metadata(b"source refusal");
    old.status = 503;
    let mut full = serde_json::to_value(&old)?;
    full["provider_capture"] = serde_json::json!({"request_id": "retained-request"});
    let encoded = serde_json::to_vec(&full)?;
    std::fs::write(&body_path, b"source refusal")?;
    std::fs::write(&meta_path, &encoded)?;
    let replacement = metadata(b"new success");
    write_cache(&body_path, &meta_path, b"new success", &replacement)?;
    let captured = capture::Capture::from_bytes(&encoded, &meta_path)?;
    let path = root
        .path()
        .join("archive/captures")
        .join(&old.content_digest)
        .join(format!("{}.meta.json", content_digest(&captured.encoded)));
    let retained: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    check!(eq; retained, full);
    check!(eq;
        std::fs::read(archive_body(root.path(), &old.content_digest))?,
        b"source refusal"
    );
    assert_capture(root.path(), b"new success", &replacement)?;
    Ok(())
}

#[test]
fn migrating_an_identical_compact_capture_does_not_manufacture_an_audit_record() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    std::fs::write(&body_path, b"raw")?;
    std::fs::write(&meta_path, serde_json::to_vec(&meta)?)?;
    write_cache(&body_path, &meta_path, b"raw", &meta)?;
    assert_capture(root.path(), b"raw", &meta)?;
    let path = archive_meta(root.path(), &meta)?;
    check!(eq;
        file_names(path.parent().ok_or("captures")?)?,
        vec![path.file_name().ok_or("name")?.to_owned()]
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn body_storage_is_shared_read_only_and_replacement_never_mutates_the_old_inode() -> TestResult {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"original");
    write_cache(&body_path, &meta_path, b"original", &first)?;
    let archived = archive_body(root.path(), &first.content_digest);
    let original = std::fs::metadata(&archived)?;
    check!(eq;
        std::fs::metadata(&body_path)?.ino(),
        original.ino()
    );
    check!(original.permissions().readonly());
    write_cache(
        &body_path,
        &meta_path,
        b"replacement",
        &metadata(b"replacement"),
    )?;
    check!(eq;
        std::fs::metadata(&archived)?.ino(),
        original.ino()
    );
    check!(eq;
        std::fs::read(archived)?,
        b"original"
    );
    Ok(())
}

mod captured_smoke;
mod failures;
mod quarantine_crashes;
mod races;
mod recovery;
