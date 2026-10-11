use super::super::archive::{
    initialize_archive, preserve_capture, ArchiveAdmission,
};
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
        for thread_id in [1, 2] {
            let barrier = Arc::clone(&barrier);
            let archive = archive.clone();
            let capture = capture.clone();
            handles.push(scope.spawn(move || {
                barrier.wait();
                let root = root.path();
                let mut admission = ArchiveAdmission::begin_with_capacity(
                    root,
                    &archive,
                    capacity,
                )?;
                let stage = test_stage(
                    root,
                    &format!("stage-{thread_id}"),
                )?;
                let digest = capture.meta.content_digest.clone();
                let meta_digest = content_digest(&capture.encoded);
                let result = preserve_capture(
                    &archive,
                    &stage,
                    "new",
                    body,
                    &capture,
                    &mut admission,
                );
                (thread_id, result)
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| "thread panicked".to_string()))
            .collect::<Vec<_>>()
    });
    for result in results {
        let (thread_id, capture_result) =
            result.map_err(std::io::Error::other)?;
        match capture_result {
            Ok(_) => {}
            Err(error) => {
                return Err(
                    format!("thread {thread_id}: {error}").into(),
                );
            }
        }
    }
    assert_capture(root.path(), body, &capture.meta)?;
    Ok(())
}

#[test]
fn failed_reservation_is_released_for_later_readers() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = initialize_archive(root.path())?;
    let first_body = b"first raw body";
    let second_body = b"second raw body";
    let first = test_capture(root.path(), first_body)?;
    let second = test_capture(root.path(), second_body)?;
    let first_needed = first_body.len() as u64 + first.encoded.len() as u64;
    let second_needed = second_body.len() as u64 + second.encoded.len() as u64;
    let capacity = first_needed + second_needed - 1;
    let first_digest = first.meta.content_digest.clone();
    let first_meta_digest = content_digest(&first.encoded);
    let second_digest = second.meta.content_digest.clone();
    let second_meta_digest = content_digest(&second.encoded);
    let mut first_admission =
        ArchiveAdmission::begin_with_capacity(root.path(), &archive, capacity)?;
    first_admission.reserve_capture_sizes(
        &first_digest,
        &first_meta_digest,
        Some(first_body.len() as u64),
        Some(first.encoded.len() as u64),
    )?;
    drop(first_admission);
    let mut second_admission =
        ArchiveAdmission::begin_with_capacity(root.path(), &archive, capacity)?;
    let second_stage = test_stage(root.path(), "stage-second")?;
    let result = preserve_capture(
        &archive,
        &second_stage,
        "new",
        second_body,
        &second,
        &mut second_admission,
    );
    drop(second_admission);
    match result {
        Ok(_) => {
            assert_capture(root.path(), second_body, &second.meta)?;
        }
        Err(FetchError::ArchiveCapacity { .. }) => {
            return Err(
                "second capture was refused despite released reservation"
                    .into(),
            );
        }
        Err(error) => return Err(error.into()),
    }
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
    let mut first_admission =
        ArchiveAdmission::begin_with_capacity(root.path(), &archive, capacity)?;
    first_admission.reserve_capture_sizes(
        &first_digest,
        &first_meta_digest,
        Some(first_body.len() as u64),
        Some(first.encoded.len() as u64),
    )?;
    drop(first_admission);
    let mut second_admission =
        ArchiveAdmission::begin_with_capacity(root.path(), &archive, capacity)?;
    let result = second_admission.reserve_capture_sizes(
        &second_digest,
        &second_meta_digest,
        Some(second_body.len() as u64),
        Some(second.encoded.len() as u64),
    );
    drop(second_admission);
    match result {
        Err(FetchError::ArchiveCapacity { .. }) => Ok(()),
        Ok(_) => Err(
            "second reservation was admitted despite insufficient capacity"
                .into(),
        ),
        Err(error) => Err(error.into()),
    }
}