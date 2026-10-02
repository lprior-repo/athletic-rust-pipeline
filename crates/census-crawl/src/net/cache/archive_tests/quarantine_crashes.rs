use super::*;

fn interrupted_quarantine(root: &Path, moved_files: usize) -> (PathBuf, Vec<u8>, CacheMeta) {
    let (body_path, meta_path) = cache_paths(root);
    let original = metadata(b"valid raw");
    write_cache(&body_path, &meta_path, b"valid raw", &original).expect("original archive");
    let encoded = std::fs::read(&meta_path).expect("original metadata");
    std::fs::remove_file(&body_path).expect("detach immutable inode");
    std::fs::write(&body_path, b"false raw").expect("damaged mutable bytes");
    let stage = archive_io::new_directory(root, ".capture-stage").expect("interrupted stage");
    let evidence = quarantine::begin_quarantine(
        &stage,
        &body_path,
        &meta_path,
        (true, true),
        quarantine::Damage::BodyIntegrity,
    )
    .expect("durable reason before moves");
    if moved_files > 0 {
        quarantine::move_evidence(&body_path, &evidence.join("body"))
            .expect("body rename boundary");
    }
    if moved_files > 1 {
        quarantine::move_evidence(&meta_path, &evidence.join("meta.json"))
            .expect("metadata rename boundary");
    }
    (evidence, encoded, original)
}

#[test]
fn every_durable_quarantine_boundary_allows_fresh_publication_without_evidence_loss() {
    for moved_files in [0, 1, 2] {
        let root = tempfile::tempdir().expect("cache directory");
        let (retained, encoded, original) = interrupted_quarantine(root.path(), moved_files);
        let reason_before = std::fs::read(retained.join("reason.json")).expect("durable reason");
        let (body_path, meta_path) = cache_paths(root.path());
        assert!(read_cache(&body_path, &meta_path)
            .expect("damaged or incomplete miss")
            .is_none());
        let fresh = metadata(b"fresh response");
        write_cache(&body_path, &meta_path, b"fresh response", &fresh)
            .expect("recover interrupted quarantine");
        assert_served(root.path(), b"fresh response", &fresh);
        assert_capture(root.path(), b"valid raw", &original);
        assert_capture(root.path(), b"fresh response", &fresh);
        assert!(!archive_body(root.path(), &content_digest(b"false raw")).exists());
        assert_eq!(
            std::fs::read(retained.join("reason.json")).expect("retained reason"),
            reason_before
        );
        let bundles = quarantine_bundles(root.path());
        assert_eq!(bundles.len(), if moved_files == 2 { 1 } else { 2 });
        let bodies: Vec<_> = bundles
            .iter()
            .filter(|bundle| bundle.join("body").exists())
            .map(|bundle| std::fs::read(bundle.join("body")).expect("exact bytes"))
            .collect();
        let metadata: Vec<_> = bundles
            .iter()
            .filter(|bundle| bundle.join("meta.json").exists())
            .map(|bundle| std::fs::read(bundle.join("meta.json")).expect("exact metadata"))
            .collect();
        assert_eq!(bodies, vec![b"false raw".to_vec()]);
        assert_eq!(metadata, vec![encoded]);
        for bundle in bundles {
            let record: serde_json::Value = serde_json::from_slice(
                &std::fs::read(bundle.join("reason.json")).expect("each bundle has reason"),
            )
            .expect("record");
            assert!(record.get("content_digest").is_none());
            assert!(record.get("status").is_none());
            if bundle != retained && moved_files == 1 {
                assert_eq!(record["reason"], "incomplete_pair");
                assert_eq!(record["body_present"], false);
                assert_eq!(record["meta_present"], true);
            }
        }
    }
}

#[test]
fn crash_before_reason_publication_does_not_block_recovery_or_remove_orphan_evidence() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let encoded = serde_json::to_vec(&metadata(b"valid raw")).expect("metadata");
    std::fs::write(&body_path, b"false raw").expect("damaged body");
    std::fs::write(&meta_path, &encoded).expect("metadata");
    let quarantine = root.path().join("quarantine");
    archive_io::create_directory(&quarantine).expect("quarantine root");
    let orphan = archive_io::new_directory(&quarantine, "capture").expect("pre-reason crash");
    let stage = archive_io::new_directory(root.path(), ".capture-stage").expect("crashed stage");
    let partial_reason = stage.join("quarantine.reason.json");
    std::fs::write(&partial_reason, b"{\"version\":").expect("interrupted reason encoding");
    let fresh = metadata(b"fresh response");
    write_cache(&body_path, &meta_path, b"fresh response", &fresh).expect("new publication");
    assert_served(root.path(), b"fresh response", &fresh);
    assert_eq!(file_names(&orphan), Vec::<std::ffi::OsString>::new());
    assert_eq!(
        std::fs::read(partial_reason).expect("foreign interrupted stage retained"),
        b"{\"version\":"
    );
    let completed: Vec<_> = quarantine_bundles(root.path())
        .into_iter()
        .filter(|bundle| *bundle != orphan)
        .collect();
    assert_eq!(completed.len(), 1);
    let retained = completed.first().expect("recovery evidence");
    assert_eq!(
        std::fs::read(retained.join("body")).expect("retained body"),
        b"false raw"
    );
    assert_eq!(
        std::fs::read(retained.join("meta.json")).expect("retained metadata"),
        encoded
    );
}

#[test]
fn occupied_quarantine_destination_is_an_error_and_never_replaced() {
    let root = tempfile::tempdir().expect("cache directory");
    let source = root.path().join("source");
    let destination = root.path().join("destination");
    std::fs::write(&source, b"mutable raw bytes").expect("source");
    std::fs::write(&destination, b"existing evidence").expect("destination");
    let result = quarantine::move_evidence(&source, &destination);
    assert!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    assert_eq!(
        std::fs::read(source).expect("source retained"),
        b"mutable raw bytes"
    );
    assert_eq!(
        std::fs::read(destination).expect("destination retained"),
        b"existing evidence"
    );
}

#[test]
fn interrupted_mutable_metadata_rename_does_not_hide_immutable_archive_corruption() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"source A");
    let second = metadata(b"source B");
    write_cache(&body_path, &meta_path, b"source A", &first).expect("first capture");
    let encoded = std::fs::read(&meta_path).expect("first metadata");
    write_cache(&body_path, &meta_path, b"source B", &second).expect("second capture");
    std::fs::write(&meta_path, &encoded).expect("old metadata after replacement");
    let fresh = metadata(b"source C");
    let collision = archive_body(root.path(), &fresh.content_digest);
    std::fs::write(&collision, b"bad data").expect("corrupt immutable content address");
    let result = write_cache(&body_path, &meta_path, b"source C", &fresh);
    assert!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    assert_eq!(
        std::fs::read(&collision).expect("no overwrite"),
        b"bad data"
    );
    assert!(!archive_meta(root.path(), &fresh).exists());
    assert_capture(root.path(), b"source A", &first);
    assert_capture(root.path(), b"source B", &second);
    assert_quarantined(
        root.path(),
        Some(b"source B"),
        Some(&encoded),
        "body_integrity",
    );
    assert!(!body_path.exists());
    assert!(!meta_path.exists());
}
