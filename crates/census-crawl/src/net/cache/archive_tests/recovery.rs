use super::*;

#[test]
fn interrupted_body_replacement_recovers_and_retains_all_three_verified_captures() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"source A");
    let second = metadata(b"source B");
    let third = metadata(b"source C");
    write_cache(&body_path, &meta_path, b"source A", &first).expect("capture A");
    let original_meta = std::fs::read(&meta_path).expect("metadata A");
    write_cache(&body_path, &meta_path, b"source B", &second).expect("capture B");
    std::fs::write(&meta_path, &original_meta).expect("simulate old metadata after body rename");
    assert!(read_cache(&body_path, &meta_path)
        .expect("mismatched cache")
        .is_none());
    write_cache(&body_path, &meta_path, b"source C", &third).expect("fresh response recovers");
    assert_capture(root.path(), b"source A", &first);
    assert_capture(root.path(), b"source B", &second);
    assert_capture(root.path(), b"source C", &third);
    assert_served(root.path(), b"source C", &third);
    assert_quarantined(
        root.path(),
        Some(b"source B"),
        Some(&original_meta),
        "body_integrity",
    );
}

#[test]
fn malformed_and_invalid_integrity_metadata_recovers_with_exact_untrusted_evidence() {
    let malformed = b"{\"provider_capture\": [\"unterminated".to_vec();
    let mut invalid = serde_json::to_value(metadata(b"old raw")).expect("metadata");
    invalid["content_digest"] = serde_json::json!("not-a-sha");
    invalid["provider_capture"] = serde_json::json!({"unaltered": "audit evidence"});
    for encoded in [
        malformed,
        serde_json::to_vec(&invalid).expect("invalid metadata"),
    ] {
        let root = tempfile::tempdir().expect("cache directory");
        let (body_path, meta_path) = cache_paths(root.path());
        std::fs::write(&body_path, b"old raw").expect("old raw bytes");
        std::fs::write(&meta_path, &encoded).expect("invalid metadata bytes");
        assert!(read_cache(&body_path, &meta_path)
            .expect("invalid metadata misses")
            .is_none());
        let fresh = metadata(b"verified fresh");
        write_cache(&body_path, &meta_path, b"verified fresh", &fresh).expect("recover metadata");
        assert_quarantined(
            root.path(),
            Some(b"old raw"),
            Some(&encoded),
            "invalid_metadata",
        );
        assert_eq!(
            file_names(&root.path().join("archive/bodies")),
            vec![archive_body(root.path(), &fresh.content_digest)
                .file_name()
                .expect("name")
                .to_owned(),]
        );
        assert_capture(root.path(), b"verified fresh", &fresh);
        assert_served(root.path(), b"verified fresh", &fresh);
    }
}

#[cfg(unix)]
#[test]
fn oversized_sparse_mutable_body_is_renamed_without_copying_or_false_archival() {
    use std::os::unix::fs::MetadataExt;
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"declared small body");
    let encoded = serde_json::to_vec(&old).expect("metadata");
    let body = std::fs::File::create(&body_path).expect("sparse body");
    let length = 1_u64 << 40;
    body.set_len(length).expect("oversized sparse body");
    body.sync_all().expect("durable sparse body");
    drop(body);
    std::fs::write(&meta_path, &encoded).expect("declared metadata");
    let original = std::fs::metadata(&body_path).expect("old inode");
    assert!(read_cache(&body_path, &meta_path)
        .expect("bounded miss")
        .is_none());
    let fresh = metadata(b"fresh");
    write_cache(&body_path, &meta_path, b"fresh", &fresh).expect("bounded recovery");
    let bundles = quarantine_bundles(root.path());
    assert_eq!(bundles.len(), 1);
    let evidence = bundles.first().expect("bundle");
    let retained = std::fs::metadata(evidence.join("body")).expect("renamed sparse inode");
    assert_eq!(retained.ino(), original.ino());
    assert_eq!(retained.len(), length);
    assert_eq!(retained.blocks(), original.blocks());
    assert_eq!(
        std::fs::read(evidence.join("meta.json")).expect("metadata"),
        encoded
    );
    let reason: serde_json::Value =
        serde_json::from_slice(&std::fs::read(evidence.join("reason.json")).expect("reason"))
            .expect("record");
    assert_eq!(reason["reason"], "body_size_or_changed");
    assert!(!archive_body(root.path(), &old.content_digest).exists());
    assert!(!archive_meta(root.path(), &old).exists());
    assert_served(root.path(), b"fresh", &fresh);
}

#[test]
fn repeated_damage_creates_distinct_evidence_without_replacing_prior_quarantine() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let encoded = serde_json::to_vec(&metadata(b"actual original")).expect("metadata");
    std::fs::write(&body_path, b"broken original").expect("corrupt body");
    std::fs::write(&meta_path, &encoded).expect("old metadata");
    let first = metadata(b"first fresh");
    write_cache(&body_path, &meta_path, b"first fresh", &first).expect("first recovery");
    let retained = assert_quarantined(
        root.path(),
        Some(b"broken original"),
        Some(&encoded),
        "body_integrity",
    );
    let reason = std::fs::read(retained.join("reason.json")).expect("original reason");
    std::fs::remove_file(&body_path).expect("simulate missing body");
    let second = metadata(b"second fresh");
    write_cache(&body_path, &meta_path, b"second fresh", &second).expect("second recovery");
    assert_eq!(quarantine_bundles(root.path()).len(), 2);
    assert_eq!(
        std::fs::read(retained.join("body")).expect("first evidence"),
        b"broken original"
    );
    assert_eq!(
        std::fs::read(retained.join("meta.json")).expect("first metadata"),
        encoded
    );
    assert_eq!(
        std::fs::read(retained.join("reason.json")).expect("first reason"),
        reason
    );
    assert_capture(root.path(), b"first fresh", &first);
    assert_capture(root.path(), b"second fresh", &second);
    assert_served(root.path(), b"second fresh", &second);
}

#[test]
fn quarantine_io_failure_preserves_damaged_mutable_evidence_and_refuses_publication() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let encoded = serde_json::to_vec(&metadata(b"actual old")).expect("metadata");
    std::fs::write(&body_path, b"damaged old").expect("corrupt body");
    std::fs::write(&meta_path, &encoded).expect("old metadata");
    std::fs::write(root.path().join("quarantine"), b"existing obstacle")
        .expect("quarantine failure");
    let fresh = metadata(b"new verified");
    let result = write_cache(&body_path, &meta_path, b"new verified", &fresh);
    assert!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    assert_eq!(
        std::fs::read(&body_path).expect("original bytes"),
        b"damaged old"
    );
    assert_eq!(
        std::fs::read(&meta_path).expect("original metadata"),
        encoded
    );
    assert_eq!(
        std::fs::read(root.path().join("quarantine")).expect("unchanged obstacle"),
        b"existing obstacle"
    );
    assert!(!archive_body(root.path(), &fresh.content_digest).exists());
    assert!(!archive_meta(root.path(), &fresh).exists());
}
