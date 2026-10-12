use super::*;

fn seed_reservation(root: &Path, capture: &capture::Capture, bytes: &str) -> TestResult<PathBuf> {
    let directory = root.join("archive/reservations");
    std::fs::create_dir_all(&directory)?;
    let path = directory.join(format!(
        "{}-{}",
        capture.meta.content_digest,
        content_digest(&capture.encoded)
    ));
    std::fs::write(&path, bytes)?;
    Ok(path)
}

#[test]
fn abandoned_unpublished_reservation_recovers_without_charging_absent_objects() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let body = b"interrupted before publication";
    let capture = test_capture(root.path(), body)?;
    let body_bytes = u64::try_from(body.len())?;
    let meta_bytes = u64::try_from(capture.encoded.len())?;
    let capacity = body_bytes
        .checked_add(meta_bytes)
        .ok_or("capacity overflow")?;
    seed_reservation(
        root.path(),
        &capture,
        &format!("{body_bytes}\n{meta_bytes}"),
    )?;
    archive_once(root.path(), &archive, 1, body, &capture, capacity)?;
    assert_capture(root.path(), body, &capture.meta)?;
    check!(eq; file_names(&archive.join("reservations"))?.len(), 0);
    Ok(())
}

#[test]
fn interrupted_body_publication_recovers_exact_usage_and_keeps_staged_evidence() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let body = b"interrupted after body publication";
    let capture = test_capture(root.path(), body)?;
    let body_bytes = u64::try_from(body.len())?;
    let meta_bytes = u64::try_from(capture.encoded.len())?;
    let capacity = body_bytes
        .checked_add(meta_bytes)
        .ok_or("capacity overflow")?;
    seed_reservation(
        root.path(),
        &capture,
        &format!("{body_bytes}\n{meta_bytes}"),
    )?;
    let stage = test_stage(root.path(), ".capture-stage-interrupted")?;
    std::fs::write(stage.join("new.body"), body)?;
    std::fs::hard_link(
        stage.join("new.body"),
        archive_body(root.path(), &capture.meta.content_digest),
    )?;
    archive_once(root.path(), &archive, 1, body, &capture, capacity)?;
    assert_capture(root.path(), body, &capture.meta)?;
    check!(eq; std::fs::read(stage.join("new.body"))?, body);
    check!(eq; file_names(&archive.join("reservations"))?.len(), 0);
    archive_once(root.path(), &archive, 2, body, &capture, capacity)?;
    Ok(())
}

#[test]
fn malformed_or_oversized_reservations_refuse_recovery_without_deleting_evidence() -> TestResult {
    for record in [
        "".to_owned(),
        "12".to_owned(),
        "12\n3\n4".to_owned(),
        "12\n-1".to_owned(),
        "0".repeat(42),
    ] {
        let root = tempfile::tempdir()?;
        let archive = initialize_archive(root.path())?;
        let capture = test_capture(root.path(), b"raw")?;
        let path = seed_reservation(root.path(), &capture, &record)?;
        let result = ArchiveAdmission::begin_with_capacity(root.path(), &archive, 4096);
        check!(matches!(result, Err(FetchError::Cache { source, .. })
            if source.kind() == ErrorKind::InvalidData));
        check!(eq; std::fs::read(path)?, record.as_bytes());
    }
    Ok(())
}

#[test]
fn symbolic_link_reservations_refuse_accounting_without_following_the_link() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    std::fs::create_dir(archive.join("reservations"))?;
    let target = root.path().join("external-record");
    std::fs::write(&target, b"10\n20")?;
    std::os::unix::fs::symlink(&target, archive.join("reservations/foreign"))?;
    let result = ArchiveAdmission::begin_with_capacity(root.path(), &archive, 4096);
    check!(matches!(result, Err(FetchError::Cache { source, .. })
        if source.kind() == ErrorKind::InvalidData));
    check!(eq; std::fs::read(target)?, b"10\n20");
    Ok(())
}
