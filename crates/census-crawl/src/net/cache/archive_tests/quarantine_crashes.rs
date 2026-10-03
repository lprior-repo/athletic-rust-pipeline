use super::*;

fn interrupted_quarantine(
    root: &Path,
    moved_files: usize,
) -> TestResult<(PathBuf, Vec<u8>, CacheMeta)> {
    let (body_path, meta_path) = cache_paths(root);
    let original = metadata(b"valid raw");
    write_cache(&body_path, &meta_path, b"valid raw", &original)?;
    let encoded = std::fs::read(&meta_path)?;
    std::fs::remove_file(&body_path)?;
    std::fs::write(&body_path, b"false raw")?;
    let stage = archive_io::new_directory(root, ".capture-stage")?;
    let evidence = quarantine::begin_quarantine(
        &stage,
        &body_path,
        &meta_path,
        (true, true),
        quarantine::Damage::BodyIntegrity,
    )?;
    if moved_files > 0 {
        quarantine::move_evidence(&body_path, &evidence.join("body"))?;
    }
    if moved_files > 1 {
        quarantine::move_evidence(&meta_path, &evidence.join("meta.json"))?;
    }
    Ok((evidence, encoded, original))
}

#[test]
fn every_durable_quarantine_boundary_allows_fresh_publication_without_evidence_loss() -> TestResult
{
    for moved_files in [0, 1, 2] {
        let root = tempfile::tempdir()?;
        let (retained, encoded, original) = interrupted_quarantine(root.path(), moved_files)?;
        let reason_before = std::fs::read(retained.join("reason.json"))?;
        let (body_path, meta_path) = cache_paths(root.path());
        check!(read_cache(&body_path, &meta_path)?.is_none());
        let fresh = metadata(b"fresh response");
        write_cache(&body_path, &meta_path, b"fresh response", &fresh)?;
        assert_served(root.path(), b"fresh response", &fresh)?;
        assert_capture(root.path(), b"valid raw", &original)?;
        assert_capture(root.path(), b"fresh response", &fresh)?;
        check!(!archive_body(root.path(), &content_digest(b"false raw")).exists());
        check!(eq;
            std::fs::read(retained.join("reason.json"))?,
            reason_before
        );
        let bundles = quarantine_bundles(root.path())?;
        check!(eq; bundles.len(), if moved_files == 2 { 1 } else { 2 });
        let bodies: Vec<_> = bundles
            .iter()
            .filter(|bundle| bundle.join("body").exists())
            .map(|bundle| std::fs::read(bundle.join("body")))
            .collect::<std::io::Result<_>>()?;
        let metadata: Vec<_> = bundles
            .iter()
            .filter(|bundle| bundle.join("meta.json").exists())
            .map(|bundle| std::fs::read(bundle.join("meta.json")))
            .collect::<std::io::Result<_>>()?;
        check!(eq; bodies, vec![b"false raw".to_vec()]);
        check!(eq; metadata, vec![encoded]);
        for bundle in bundles {
            let record: serde_json::Value =
                serde_json::from_slice(&std::fs::read(bundle.join("reason.json"))?)?;
            check!(record.get("content_digest").is_none());
            check!(record.get("status").is_none());
            if bundle != retained && moved_files == 1 {
                check!(eq; record["reason"], "incomplete_pair");
                check!(eq; record["body_present"], false);
                check!(eq; record["meta_present"], true);
            }
        }
    }
    Ok(())
}

#[test]
fn crash_before_reason_publication_does_not_block_recovery_or_remove_orphan_evidence() -> TestResult
{
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let encoded = serde_json::to_vec(&metadata(b"valid raw"))?;
    std::fs::write(&body_path, b"false raw")?;
    std::fs::write(&meta_path, &encoded)?;
    let quarantine = root.path().join("quarantine");
    archive_io::create_directory(&quarantine)?;
    let orphan = archive_io::new_directory(&quarantine, "capture")?;
    let stage = archive_io::new_directory(root.path(), ".capture-stage")?;
    let partial_reason = stage.join("quarantine.reason.json");
    std::fs::write(&partial_reason, b"{\"version\":")?;
    let fresh = metadata(b"fresh response");
    write_cache(&body_path, &meta_path, b"fresh response", &fresh)?;
    assert_served(root.path(), b"fresh response", &fresh)?;
    check!(eq; file_names(&orphan)?, Vec::<std::ffi::OsString>::new());
    check!(eq;
        std::fs::read(partial_reason)?,
        b"{\"version\":"
    );
    let completed: Vec<_> = quarantine_bundles(root.path())?
        .into_iter()
        .filter(|bundle| *bundle != orphan)
        .collect();
    check!(eq; completed.len(), 1);
    let retained = completed.first().ok_or("recovery evidence")?;
    check!(eq;
        std::fs::read(retained.join("body"))?,
        b"false raw"
    );
    check!(eq;
        std::fs::read(retained.join("meta.json"))?,
        encoded
    );
    Ok(())
}

#[test]
fn occupied_quarantine_destination_is_an_error_and_never_replaced() -> TestResult {
    let root = tempfile::tempdir()?;
    let source = root.path().join("source");
    let destination = root.path().join("destination");
    std::fs::write(&source, b"mutable raw bytes")?;
    std::fs::write(&destination, b"existing evidence")?;
    let result = quarantine::move_evidence(&source, &destination);
    check!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    check!(eq;
        std::fs::read(source)?,
        b"mutable raw bytes"
    );
    check!(eq;
        std::fs::read(destination)?,
        b"existing evidence"
    );
    Ok(())
}

#[test]
fn interrupted_mutable_metadata_rename_does_not_hide_immutable_archive_corruption() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let first = metadata(b"source A");
    let second = metadata(b"source B");
    write_cache(&body_path, &meta_path, b"source A", &first)?;
    let encoded = std::fs::read(&meta_path)?;
    write_cache(&body_path, &meta_path, b"source B", &second)?;
    std::fs::write(&meta_path, &encoded)?;
    let fresh = metadata(b"source C");
    let collision = archive_body(root.path(), &fresh.content_digest);
    std::fs::write(&collision, b"bad data")?;
    let result = write_cache(&body_path, &meta_path, b"source C", &fresh);
    check!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData)
    );
    check!(eq;
        std::fs::read(&collision)?,
        b"bad data"
    );
    check!(!archive_meta(root.path(), &fresh)?.exists());
    assert_capture(root.path(), b"source A", &first)?;
    assert_capture(root.path(), b"source B", &second)?;
    assert_quarantined(
        root.path(),
        Some(b"source B"),
        Some(&encoded),
        "body_integrity",
    )?;
    check!(!body_path.exists());
    check!(!meta_path.exists());
    Ok(())
}
