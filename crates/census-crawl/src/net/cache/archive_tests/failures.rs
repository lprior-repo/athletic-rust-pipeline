use super::*;

#[test]
fn mismatched_old_body_is_never_archived_under_its_declared_hash() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"true raw");
    std::fs::write(&body_path, b"fake raw")?;
    let encoded = serde_json::to_vec(&old)?;
    std::fs::write(&meta_path, &encoded)?;
    let fresh = metadata(b"new raw");
    write_cache(&body_path, &meta_path, b"new raw", &fresh)?;
    check!(!archive_body(root.path(), &old.content_digest).exists());
    check!(!archive_meta(root.path(), &old)?.exists());
    check!(!archive_body(root.path(), &content_digest(b"fake raw")).exists());
    assert_quarantined(
        root.path(),
        Some(b"fake raw"),
        Some(&encoded),
        "body_integrity",
    )?;
    assert_capture(root.path(), b"new raw", &fresh)?;
    assert_served(root.path(), b"new raw", &fresh)?;
    Ok(())
}

#[test]
fn incomplete_old_capture_recovers_without_claiming_false_preservation() -> TestResult {
    for body_present in [true, false] {
        let root = tempfile::tempdir()?;
        let (body_path, meta_path) = cache_paths(root.path());
        let old = metadata(b"old raw");
        let encoded = serde_json::to_vec(&old)?;
        if body_present {
            std::fs::write(&body_path, b"old raw")?;
        } else {
            std::fs::write(&meta_path, &encoded)?;
        }
        let fresh = metadata(b"new raw");
        write_cache(&body_path, &meta_path, b"new raw", &fresh)?;
        check!(!archive_body(root.path(), &old.content_digest).exists());
        check!(!archive_meta(root.path(), &old)?.exists());
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
        assert_quarantined(root.path(), body, meta, "incomplete_pair")?;
        assert_capture(root.path(), b"new raw", &fresh)?;
        assert_served(root.path(), b"new raw", &fresh)?;
    }
    Ok(())
}

#[test]
fn invalid_new_digest_length_and_oversized_metadata_do_not_publish_any_capture() -> TestResult {
    let root = tempfile::tempdir()?;
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
        check!(matches!(
            result,
            Err(FetchError::Cache { .. } | FetchError::Encode { .. })
        ));
        check!(!body_path.exists());
        check!(!meta_path.exists());
        check!(!root.path().join("archive").exists());
    }
    Ok(())
}

#[test]
fn archive_io_failure_keeps_the_verified_old_mutable_cache_unreplaced() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"old raw");
    let encoded = serde_json::to_vec(&old)?;
    std::fs::write(&body_path, b"old raw")?;
    std::fs::write(&meta_path, &encoded)?;
    std::fs::write(root.path().join("archive"), b"not a directory")?;
    let result = write_cache(&body_path, &meta_path, b"new raw", &metadata(b"new raw"));
    check!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    check!(eq; std::fs::read(&body_path)?, b"old raw");
    check!(eq; std::fs::read(&meta_path)?, encoded);
    Ok(())
}

#[test]
fn corrupt_existing_archive_body_and_metadata_each_refuse_false_success() -> TestResult {
    for corrupt_body in [true, false] {
        let root = tempfile::tempdir()?;
        let (body_path, meta_path) = cache_paths(root.path());
        let meta = metadata(b"raw");
        let body = archive_body(root.path(), &meta.content_digest);
        let capture_meta = archive_meta(root.path(), &meta)?;
        std::fs::create_dir_all(body.parent().ok_or("bodies")?)?;
        std::fs::create_dir_all(capture_meta.parent().ok_or("captures")?)?;
        std::fs::write(&body, if corrupt_body { b"bad" } else { b"raw" })?;
        if !corrupt_body {
            std::fs::write(&capture_meta, b"{}")?;
        }
        let result = write_cache(&body_path, &meta_path, b"raw", &meta);
        check!(
            matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
        );
        check!(!body_path.exists());
        check!(!meta_path.exists());
        if corrupt_body {
            check!(!capture_meta.exists());
            check!(eq; std::fs::read(&body)?, b"bad");
        } else {
            check!(eq; std::fs::read(&capture_meta)?, b"{}");
        }
    }
    Ok(())
}

#[test]
fn body_only_archive_publication_resumes_but_another_runs_stage_is_never_removed() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    let body = archive_body(root.path(), &meta.content_digest);
    std::fs::create_dir_all(body.parent().ok_or("bodies")?)?;
    std::fs::write(&body, b"raw")?;
    let foreign_stage = root.path().join(".capture-stage-other-run");
    std::fs::create_dir(&foreign_stage)?;
    let sentinel = foreign_stage.join("capture.body");
    std::fs::write(&sentinel, b"other run evidence")?;
    write_cache(&body_path, &meta_path, b"raw", &meta)?;
    assert_capture(root.path(), b"raw", &meta)?;
    check!(eq;
        std::fs::read(sentinel)?,
        b"other run evidence"
    );
    Ok(())
}

#[test]
fn metadata_publication_io_failure_leaves_no_complete_archive_or_cache() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    let target = archive_meta(root.path(), &meta)?;
    std::fs::create_dir_all(&target)?;
    let result = write_cache(&body_path, &meta_path, b"raw", &meta);
    check!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    check!(eq;
        std::fs::read(archive_body(root.path(), &meta.content_digest))?,
        b"raw"
    );
    check!(target.is_dir());
    check!(!body_path.exists());
    check!(!meta_path.exists());
    Ok(())
}
