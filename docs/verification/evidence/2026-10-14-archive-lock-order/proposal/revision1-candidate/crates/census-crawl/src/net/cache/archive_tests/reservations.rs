use super::super::archive::admission::ArchiveCapacity;
use super::super::archive::{initialize_archive, preserve_capture, ArchiveAdmission, ArchiveLock};
use super::*;
use std::sync::{Arc, Barrier};

fn test_capture(root: &Path, body: &[u8]) -> TestResult<capture::Capture> {
    let (_, meta_path) = cache_paths(root);
    Ok(capture::Capture::from_meta(&metadata(body), &meta_path)?)
}

fn test_stage(root: &Path, name: &str) -> TestResult<PathBuf> {
    let stage = root.join(name);
    std::fs::create_dir(&stage)?;
    Ok(stage)
}

fn archive_once(
    root: &Path,
    archive: &Path,
    stage_name: u32,
    body: &[u8],
    capture: &capture::Capture,
    capacity: u64,
) -> TestResult {
    let capacity_admission = ArchiveCapacity::new(capacity, archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root)?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    let stage = test_stage(root, &format!("stage-{stage_name}"))?;
    preserve_capture(archive, &stage, "new", body, capture, &mut admission)?;
    Ok(())
}

#[test]
fn concurrent_identical_reservations_are_deduplicated() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let body = b"identical raw body";
    let capture = test_capture(root.path(), body)?;
    let body_bytes = body.len() as u64;
    let meta_bytes = capture.encoded.len() as u64;
    let capacity = body_bytes + meta_bytes;
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for thread_id in [1_u32, 2] {
            let barrier = Arc::clone(&barrier);
            let archive = archive.clone();
            let capture = capture.clone();
            let root_dir = root.path().to_path_buf();
            handles.push(scope.spawn(move || {
                barrier.wait();
                archive_once(&root_dir, &archive, thread_id, body, &capture, capacity)
                    .map_err(|error| error.to_string())
            }));
        }
        handles
            .into_iter()
            .map(|handle| match handle.join() {
                Ok(result) => result,
                Err(_) => Err("thread panicked".to_string()),
            })
            .collect::<Vec<_>>()
    });
    for result in results {
        result.map_err(std::io::Error::other)?;
    }
    assert_capture(root.path(), body, &capture.meta)?;
    Ok(())
}

#[test]
fn published_capture_with_stale_reservation_is_recovered_on_restart() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let first_body = b"first raw body";
    let second_body = b"second raw body";
    let first = test_capture(root.path(), first_body)?;
    let second = test_capture(root.path(), second_body)?;
    let first_needed = first_body.len() as u64 + first.encoded.len() as u64;
    let second_needed = second_body.len() as u64 + second.encoded.len() as u64;
    let capacity = first_needed + second_needed;
    let capacity_first_admission = ArchiveCapacity::new(capacity, &archive)?;
    let archive_lock_first_admission = ArchiveLock::acquire(root.path())?;
    let mut first_admission = ArchiveAdmission::begin_with_capacity(&archive_lock_first_admission, capacity_first_admission)?;
    let first_stage = test_stage(root.path(), "stage-first")?;
    preserve_capture(
        &archive,
        &first_stage,
        "new",
        first_body,
        &first,
        &mut first_admission,
    )?;
    drop(first_admission);
    drop(archive_lock_first_admission);
    let capacity_abandoned = ArchiveCapacity::new(capacity, &archive)?;
    let archive_lock_abandoned = ArchiveLock::acquire(root.path())?;
    let mut abandoned = ArchiveAdmission::begin_with_capacity(&archive_lock_abandoned, capacity_abandoned)?;
    abandoned.reserve_capture_sizes(
        &first.meta.content_digest,
        &content_digest(&first.encoded),
        Some(first_body.len() as u64),
        Some(first.encoded.len() as u64),
    )?;
    drop(abandoned);
    drop(archive_lock_abandoned);
    let capacity_second_admission = ArchiveCapacity::new(capacity, &archive)?;
    let archive_lock_second_admission = ArchiveLock::acquire(root.path())?;
    let mut second_admission = ArchiveAdmission::begin_with_capacity(&archive_lock_second_admission, capacity_second_admission)?;
    let second_stage = test_stage(root.path(), "stage-second")?;
    preserve_capture(
        &archive,
        &second_stage,
        "new",
        second_body,
        &second,
        &mut second_admission,
    )?;
    assert_capture(root.path(), second_body, &second.meta)?;
    Ok(())
}
#[test]
fn reserved_bytes_are_counted_and_refuse_competing_captures() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let first_body = b"first raw body content";
    let second_body = b"second raw body content";
    let first = test_capture(root.path(), first_body)?;
    let second = test_capture(root.path(), second_body)?;
    let first_needed = first_body.len() as u64 + first.encoded.len() as u64;
    let second_needed = second_body.len() as u64 + second.encoded.len() as u64;
    let capacity = first_needed + second_needed - 1;
    let first_digest = first.meta.content_digest.clone();
    let first_meta_digest = content_digest(&first.encoded);
    let second_digest = second.meta.content_digest.clone();
    let second_meta_digest = content_digest(&second.encoded);
    let capacity_first_admission = ArchiveCapacity::new(capacity, &archive)?;
    let archive_lock_first_admission = ArchiveLock::acquire(root.path())?;
    let mut first_admission = ArchiveAdmission::begin_with_capacity(&archive_lock_first_admission, capacity_first_admission)?;
    first_admission.reserve_capture_sizes(
        &first_digest,
        &first_meta_digest,
        Some(first_body.len() as u64),
        Some(first.encoded.len() as u64),
    )?;
    let result = first_admission.reserve_capture_sizes(
        &second_digest,
        &second_meta_digest,
        Some(second_body.len() as u64),
        Some(second.encoded.len() as u64),
    );
    drop(first_admission);
    drop(archive_lock_first_admission);
    match result {
        Err(FetchError::ArchiveCapacity { .. }) => Ok(()),
        Ok(_) => Err("second reservation was admitted despite insufficient capacity".into()),
        Err(error) => Err(error.into()),
    }
}

#[test]
fn repeated_in_flight_archive_key_reserves_its_bytes_only_once() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let body = b"single reservation";
    let capture = test_capture(root.path(), body)?;
    let body_bytes = u64::try_from(body.len())?;
    let meta_bytes = u64::try_from(capture.encoded.len())?;
    let capacity = body_bytes
        .checked_add(meta_bytes)
        .ok_or("capacity overflow")?;
    let digest = content_digest(&capture.encoded);
    let capacity_admission = ArchiveCapacity::new(capacity, &archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root.path())?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    admission.reserve_capture_sizes(
        &capture.meta.content_digest,
        &digest,
        Some(body_bytes),
        Some(meta_bytes),
    )?;
    admission.reserve_capture_sizes(
        &capture.meta.content_digest,
        &digest,
        Some(body_bytes),
        Some(meta_bytes),
    )?;
    check!(eq; file_names(&archive.join("reservations"))?.len(), 1);
    admission.finish_capture(&capture.meta.content_digest, &digest)?;
    check!(eq; file_names(&archive.join("reservations"))?.len(), 0);
    Ok(())
}

#[test]
fn failed_staged_write_releases_reservation_before_admission_is_dropped() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let body = b"failed staged capture";
    let capture = test_capture(root.path(), body)?;
    let capacity = u64::try_from(body.len())?
        .checked_add(u64::try_from(capture.encoded.len())?)
        .ok_or("capacity overflow")?;
    let capacity_admission = ArchiveCapacity::new(capacity, &archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root.path())?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    let stage = test_stage(root.path(), "stage-obstructed")?;
    let obstruction = stage.join("new.body");
    std::fs::write(&obstruction, b"retained interrupted bytes")?;
    let result = preserve_capture(&archive, &stage, "new", body, &capture, &mut admission);
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path == obstruction && source.kind() == ErrorKind::AlreadyExists));
    check!(eq; std::fs::read(obstruction)?, b"retained interrupted bytes");
    check!(eq; file_names(&archive.join("reservations"))?.len(), 0);
    let retry_stage = test_stage(root.path(), "stage-retry")?;
    preserve_capture(
        &archive,
        &retry_stage,
        "new",
        body,
        &capture,
        &mut admission,
    )?;
    assert_capture(root.path(), body, &capture.meta)?;
    Ok(())
}

mod recovery;
