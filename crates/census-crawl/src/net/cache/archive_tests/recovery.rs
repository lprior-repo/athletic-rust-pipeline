use super::*;

#[test]
fn interrupted_body_replacement_recovers_and_retains_all_three_verified_captures() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"source A");
    let second = metadata(b"source B");
    let third = metadata(b"source C");
    write_cache(&body_path, &meta_path, b"source A", &first)?;
    let original_meta = std::fs::read(&meta_path)?;
    write_cache(&body_path, &meta_path, b"source B", &second)?;
    std::fs::write(&meta_path, &original_meta)?;
    check!(read_cache(&body_path, &meta_path)?.is_none());
    write_cache(&body_path, &meta_path, b"source C", &third)?;
    assert_capture(root.path(), b"source A", &first)?;
    assert_capture(root.path(), b"source B", &second)?;
    assert_capture(root.path(), b"source C", &third)?;
    assert_served(root.path(), b"source C", &third)?;
    assert_quarantined(
        root.path(),
        Some(b"source B"),
        Some(&original_meta),
        "body_integrity",
    )?;
    Ok(())
}

#[test]
fn malformed_and_invalid_integrity_metadata_recovers_with_exact_untrusted_evidence() -> TestResult {
    let malformed = b"{\"provider_capture\": [\"unterminated".to_vec();
    let mut invalid = serde_json::to_value(metadata(b"old raw"))?;
    invalid["content_digest"] = serde_json::json!("not-a-sha");
    invalid["provider_capture"] = serde_json::json!({"unaltered": "audit evidence"});
    for encoded in [malformed, serde_json::to_vec(&invalid)?] {
        let root = tempfile::tempdir()?;
        let (body_path, meta_path) = cache_paths(root.path());
        std::fs::write(&body_path, b"old raw")?;
        std::fs::write(&meta_path, &encoded)?;
        check!(read_cache(&body_path, &meta_path)?.is_none());
        let fresh = metadata(b"verified fresh");
        write_cache(&body_path, &meta_path, b"verified fresh", &fresh)?;
        assert_quarantined(
            root.path(),
            Some(b"old raw"),
            Some(&encoded),
            "invalid_metadata",
        )?;
        check!(eq;
            file_names(&root.path().join("archive/bodies"))?,
            vec![archive_body(root.path(), &fresh.content_digest)
                .file_name()
                .ok_or("name")?
                .to_owned(),]
        );
        assert_capture(root.path(), b"verified fresh", &fresh)?;
        assert_served(root.path(), b"verified fresh", &fresh)?;
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn oversized_sparse_mutable_body_is_renamed_without_copying_or_false_archival() -> TestResult {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"declared small body");
    let encoded = serde_json::to_vec(&old)?;
    let body = std::fs::File::create(&body_path)?;
    let length = 1_u64 << 40;
    body.set_len(length)?;
    body.sync_all()?;
    drop(body);
    std::fs::write(&meta_path, &encoded)?;
    let original = std::fs::metadata(&body_path)?;
    check!(read_cache(&body_path, &meta_path)?.is_none());
    let fresh = metadata(b"fresh");
    write_cache(&body_path, &meta_path, b"fresh", &fresh)?;
    let bundles = quarantine_bundles(root.path())?;
    check!(eq; bundles.len(), 1);
    let evidence = bundles.first().ok_or("bundle")?;
    let retained = std::fs::metadata(evidence.join("body"))?;
    check!(eq; retained.ino(), original.ino());
    check!(eq; retained.len(), length);
    check!(eq; retained.blocks(), original.blocks());
    check!(eq;
        std::fs::read(evidence.join("meta.json"))?,
        encoded
    );
    let reason: serde_json::Value =
        serde_json::from_slice(&std::fs::read(evidence.join("reason.json"))?)?;
    check!(eq; reason["reason"], "body_size_or_changed");
    check!(!archive_body(root.path(), &old.content_digest).exists());
    check!(!archive_meta(root.path(), &old)?.exists());
    assert_served(root.path(), b"fresh", &fresh)?;
    Ok(())
}

#[test]
fn repeated_damage_creates_distinct_evidence_without_replacing_prior_quarantine() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let encoded = serde_json::to_vec(&metadata(b"actual original"))?;
    std::fs::write(&body_path, b"broken original")?;
    std::fs::write(&meta_path, &encoded)?;
    let first = metadata(b"first fresh");
    write_cache(&body_path, &meta_path, b"first fresh", &first)?;
    let retained = assert_quarantined(
        root.path(),
        Some(b"broken original"),
        Some(&encoded),
        "body_integrity",
    )?;
    let reason = std::fs::read(retained.join("reason.json"))?;
    std::fs::remove_file(&body_path)?;
    let second = metadata(b"second fresh");
    write_cache(&body_path, &meta_path, b"second fresh", &second)?;
    check!(eq; quarantine_bundles(root.path())?.len(), 2);
    check!(eq;
        std::fs::read(retained.join("body"))?,
        b"broken original"
    );
    check!(eq;
        std::fs::read(retained.join("meta.json"))?,
        encoded
    );
    check!(eq;
        std::fs::read(retained.join("reason.json"))?,
        reason
    );
    assert_capture(root.path(), b"first fresh", &first)?;
    assert_capture(root.path(), b"second fresh", &second)?;
    assert_served(root.path(), b"second fresh", &second)?;
    Ok(())
}

#[test]
fn quarantine_io_failure_preserves_damaged_mutable_evidence_and_refuses_publication() -> TestResult
{
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let encoded = serde_json::to_vec(&metadata(b"actual old"))?;
    std::fs::write(&body_path, b"damaged old")?;
    std::fs::write(&meta_path, &encoded)?;
    std::fs::write(root.path().join("quarantine"), b"existing obstacle")?;
    let fresh = metadata(b"new verified");
    let result = write_cache(&body_path, &meta_path, b"new verified", &fresh);
    check!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    check!(eq;
        std::fs::read(&body_path)?,
        b"damaged old"
    );
    check!(eq;
        std::fs::read(&meta_path)?,
        encoded
    );
    check!(eq;
        std::fs::read(root.path().join("quarantine"))?,
        b"existing obstacle"
    );
    check!(!archive_body(root.path(), &fresh.content_digest).exists());
    check!(!archive_meta(root.path(), &fresh)?.exists());
    Ok(())
}
