use super::*;

#[test]
fn mismatched_old_body_is_never_archived_under_its_declared_hash() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"true raw");
    std::fs::write(&body_path, b"fake raw").expect("corrupt body");
    let encoded = serde_json::to_vec(&old).expect("metadata");
    std::fs::write(&meta_path, &encoded).expect("old metadata");
    let fresh = metadata(b"new raw");
    write_cache(&body_path, &meta_path, b"new raw", &fresh).expect("recover corrupted cache");
    assert!(!archive_body(root.path(), &old.content_digest).exists());
    assert!(!archive_meta(root.path(), &old).exists());
    assert!(!archive_body(root.path(), &content_digest(b"fake raw")).exists());
    assert_quarantined(
        root.path(),
        Some(b"fake raw"),
        Some(&encoded),
        "body_integrity",
    );
    assert_capture(root.path(), b"new raw", &fresh);
    assert_served(root.path(), b"new raw", &fresh);
}

#[test]
fn incomplete_old_capture_recovers_without_claiming_false_preservation() {
    for body_present in [true, false] {
        let root = tempfile::tempdir().expect("cache directory");
        let (body_path, meta_path) = cache_paths(root.path());
        let old = metadata(b"old raw");
        let encoded = serde_json::to_vec(&old).expect("metadata");
        if body_present {
            std::fs::write(&body_path, b"old raw").expect("old body only");
        } else {
            std::fs::write(&meta_path, &encoded).expect("metadata only");
        }
        let fresh = metadata(b"new raw");
        write_cache(&body_path, &meta_path, b"new raw", &fresh).expect("recover missing pair");
        assert!(!archive_body(root.path(), &old.content_digest).exists());
        assert!(!archive_meta(root.path(), &old).exists());
        let body = if body_present {
            Some(b"old raw".as_slice())
        } else {
            None
        };
        let meta = if body_present {
            None
        } else {
            Some(encoded.as_slice())
        };
        assert_quarantined(root.path(), body, meta, "incomplete_pair");
        assert_capture(root.path(), b"new raw", &fresh);
        assert_served(root.path(), b"new raw", &fresh);
    }
}

#[test]
fn invalid_new_digest_length_and_oversized_metadata_do_not_publish_any_capture() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let mut wrong_hash = metadata(b"new raw");
    wrong_hash.content_digest = content_digest(b"other");
    let mut wrong_length = metadata(b"new raw");
    wrong_length.bytes += 1;
    let mut oversized = metadata(b"new raw");
    oversized.url = "x".repeat(MAX_META_BYTES + 1);
    let mut escaped = metadata(b"new raw");
    escaped.etag = Some("\n".repeat(MAX_META_BYTES / 2));
    let mut body_over_limit = metadata(b"new raw");
    body_over_limit.bytes = MAX_BODY_BYTES + 1;
    for meta in [
        wrong_hash,
        wrong_length,
        oversized,
        escaped,
        body_over_limit,
    ] {
        let result = write_cache(&body_path, &meta_path, b"new raw", &meta);
        assert!(matches!(
            result,
            Err(FetchError::Cache { .. } | FetchError::Encode { .. })
        ));
        assert!(!body_path.exists());
        assert!(!meta_path.exists());
        assert!(!root.path().join("archive").exists());
    }
}

#[test]
fn archive_io_failure_keeps_the_verified_old_mutable_cache_unreplaced() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"old raw");
    let encoded = serde_json::to_vec(&old).expect("metadata");
    std::fs::write(&body_path, b"old raw").expect("old body");
    std::fs::write(&meta_path, &encoded).expect("old metadata");
    std::fs::write(root.path().join("archive"), b"not a directory").expect("archive fault");
    let result = write_cache(&body_path, &meta_path, b"new raw", &metadata(b"new raw"));
    assert!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    assert_eq!(std::fs::read(&body_path).expect("old body"), b"old raw");
    assert_eq!(std::fs::read(&meta_path).expect("old metadata"), encoded);
}

#[test]
fn corrupt_existing_archive_body_and_metadata_each_refuse_false_success() {
    for corrupt_body in [true, false] {
        let root = tempfile::tempdir().expect("cache directory");
        let (body_path, meta_path) = cache_paths(root.path());
        let meta = metadata(b"raw");
        let body = archive_body(root.path(), &meta.content_digest);
        let capture_meta = archive_meta(root.path(), &meta);
        std::fs::create_dir_all(body.parent().expect("bodies")).expect("archive bodies");
        std::fs::create_dir_all(capture_meta.parent().expect("captures"))
            .expect("archive captures");
        std::fs::write(&body, if corrupt_body { b"bad" } else { b"raw" }).expect("archive body");
        if !corrupt_body {
            std::fs::write(&capture_meta, b"{}").expect("corrupt metadata");
        }
        let result = write_cache(&body_path, &meta_path, b"raw", &meta);
        assert!(
            matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
        );
        assert!(!body_path.exists());
        assert!(!meta_path.exists());
        if corrupt_body {
            assert!(!capture_meta.exists());
            assert_eq!(std::fs::read(&body).expect("no overwrite"), b"bad");
        } else {
            assert_eq!(std::fs::read(&capture_meta).expect("no overwrite"), b"{}");
        }
    }
}

#[test]
fn body_only_archive_publication_resumes_but_another_runs_stage_is_never_removed() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    let body = archive_body(root.path(), &meta.content_digest);
    std::fs::create_dir_all(body.parent().expect("bodies")).expect("partial archive");
    std::fs::write(&body, b"raw").expect("durable body");
    let foreign_stage = root.path().join(".capture-stage-other-run");
    std::fs::create_dir(&foreign_stage).expect("foreign stage");
    let sentinel = foreign_stage.join("capture.body");
    std::fs::write(&sentinel, b"other run evidence").expect("foreign evidence");
    write_cache(&body_path, &meta_path, b"raw", &meta).expect("resume metadata publication");
    assert_capture(root.path(), b"raw", &meta);
    assert_eq!(
        std::fs::read(sentinel).expect("untouched foreign stage"),
        b"other run evidence"
    );
}

#[test]
fn metadata_publication_io_failure_leaves_no_complete_archive_or_cache() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    let target = archive_meta(root.path(), &meta);
    std::fs::create_dir_all(&target).expect("blocked metadata filename");
    let result = write_cache(&body_path, &meta_path, b"raw", &meta);
    assert!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    assert_eq!(
        std::fs::read(archive_body(root.path(), &meta.content_digest)).expect("body only"),
        b"raw"
    );
    assert!(target.is_dir());
    assert!(!body_path.exists());
    assert!(!meta_path.exists());
}
