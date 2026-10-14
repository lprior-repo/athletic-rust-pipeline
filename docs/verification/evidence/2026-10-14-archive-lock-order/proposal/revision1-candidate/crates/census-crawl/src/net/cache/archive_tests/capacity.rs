use super::super::archive::admission::{capacity_of, DEFAULT_ARCHIVE_BYTES};
use super::super::archive::admission::ArchiveCapacity;
use super::super::archive::{preserve_capture, ArchiveAdmission, ArchiveLock};
use super::*;

fn archive_dirs(root: &Path) -> TestResult<PathBuf> {
    let archive = root.join("archive");
    std::fs::create_dir_all(archive.join("bodies"))?;
    std::fs::create_dir_all(archive.join("captures"))?;
    Ok(archive)
}

fn archive_stage(root: &Path, name: &str) -> TestResult<PathBuf> {
    let stage = root.join(format!(".capture-stage-{name}"));
    std::fs::create_dir(&stage)?;
    Ok(stage)
}

fn capture_of(root: &Path, body: &[u8]) -> TestResult<capture::Capture> {
    let (_, meta_path) = cache_paths(root);
    Ok(capture::Capture::from_meta(&metadata(body), &meta_path)?)
}

fn needed_bytes(body: &[u8], capture: &capture::Capture) -> u64 {
    body.len() as u64 + capture.encoded.len() as u64
}

#[test]
fn configured_capacity_of_honors_positive_integers_and_defaults_only_when_unset() -> TestResult {
    check!(eq; capacity_of(Some("2048".to_string()))?.get(), 2048);
    check!(eq; capacity_of(Some("  4096 \n".to_string()))?.get(), 4096);
    check!(eq; capacity_of(None)?.get(), DEFAULT_ARCHIVE_BYTES);
    check!(capacity_of(Some("0".to_string())).is_err());
    check!(capacity_of(Some("twelve".to_string())).is_err());
    check!(capacity_of(Some("".to_string())).is_err());
    Ok(())
}

#[test]
fn a_capture_over_capacity_is_refused_before_any_object_is_published() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"capture body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let capacity_admission = ArchiveCapacity::new(needed - 1, &archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root.path())?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    let stage = archive_stage(root.path(), "stage-refused")?;
    let outcome = preserve_capture(&archive, &stage, "new", body, &capture, &mut admission);
    let Err(error) = outcome else {
        return Err("a capacity refusal should leave the capture unpublished".into());
    };
    check!(
        matches!(
            &error,
            FetchError::ArchiveCapacity { used: 0, requested, capacity, .. }
                if *requested == needed && *capacity == needed - 1
        ),
        "the refusal is typed and reports the exhausted scope: {error}"
    );
    check!(!archive_body(root.path(), &capture.meta.content_digest).exists());
    check!(!archive_meta(root.path(), &capture.meta)?.exists());
    Ok(())
}

#[test]
fn a_capture_that_fits_is_published_and_settles_the_reservation() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"capture body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let capacity_admission = ArchiveCapacity::new(needed, &archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root.path())?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    let stage = archive_stage(root.path(), "stage-fits")?;
    preserve_capture(&archive, &stage, "new", body, &capture, &mut admission)?;
    assert_capture(root.path(), body, &capture.meta)?;
    drop(admission);
    drop(archive_lock_admission);
    let capacity_replay = ArchiveCapacity::new(needed, &archive)?;
    let archive_lock_replay = ArchiveLock::acquire(root.path())?;
    let mut replay = ArchiveAdmission::begin_with_capacity(&archive_lock_replay, capacity_replay)?;
    let replay_stage = archive_stage(root.path(), "stage-replay")?;
    preserve_capture(&archive, &replay_stage, "new", body, &capture, &mut replay)?;
    Ok(())
}

#[test]
fn an_archived_capture_settles_without_a_second_reservation() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"shared raw body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let capacity_first = ArchiveCapacity::new(needed, &archive)?;
    let archive_lock_first = ArchiveLock::acquire(root.path())?;
    let mut first = ArchiveAdmission::begin_with_capacity(&archive_lock_first, capacity_first)?;
    let first_stage = archive_stage(root.path(), "stage-first")?;
    preserve_capture(&archive, &first_stage, "new", body, &capture, &mut first)?;
    drop(first);
    drop(archive_lock_first);
    let capacity_second = ArchiveCapacity::new(needed, &archive)?;
    let archive_lock_second = ArchiveLock::acquire(root.path())?;
    let mut second = ArchiveAdmission::begin_with_capacity(&archive_lock_second, capacity_second)?;
    let second_stage = archive_stage(root.path(), "stage-second")?;
    preserve_capture(&archive, &second_stage, "old", body, &capture, &mut second)?;
    check!(
        eq; file_names(&root.path().join("archive/bodies"))?.len(), 1,
        "an already-archived body is not reserved or written twice"
    );
    Ok(())
}

#[test]
fn invalid_archive_entries_refuse_capacity_accounting() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    std::os::unix::fs::symlink("bodies", archive.join("linked"))?;
    let capacity_outcome = ArchiveCapacity::new(1 << 20, &archive)?;
    let archive_lock_outcome = ArchiveLock::acquire(root.path())?;
    let outcome = ArchiveAdmission::begin_with_capacity(&archive_lock_outcome, capacity_outcome);
    let Err(error) = outcome else {
        return Err("accounting over a symbolic link should be refused".into());
    };
    check!(
        error.to_string().contains("symbolic link"),
        "the inventory failure names the offending entry: {error}"
    );
    Ok(())
}

#[test]
fn concurrent_publishers_share_one_capacity_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let first_body = b"first raw!!";
    let second_body = b"second raw!";
    let first = capture_of(root.path(), first_body)?;
    let second = capture_of(root.path(), second_body)?;
    check!(eq; first_body.len(), second_body.len());
    check!(eq; first.encoded.len(), second.encoded.len());
    let capacity = needed_bytes(first_body, &first) * 2 - 1;
    let stages: Vec<PathBuf> = ["stage-0", "stage-1"]
        .into_iter()
        .map(|name| archive_stage(root.path(), name))
        .collect::<TestResult<_>>()?;
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = [
            (first_body.as_slice(), &first),
            (second_body.as_slice(), &second),
        ]
        .into_iter()
        .zip(stages)
        .map(|((body, capture), stage)| {
            let root = root.path();
            let archive = archive.clone();
            scope.spawn(move || {
                let capacity_admission = ArchiveCapacity::new(capacity, &archive)?;
                let archive_lock_admission = ArchiveLock::acquire(root)?;
                let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
                preserve_capture(&archive, &stage, "new", body, capture, &mut admission)
            })
        })
        .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "publisher thread panicked".to_string())
            })
            .collect::<Vec<_>>()
    });
    let mut published = 0;
    let mut refused = 0;
    for result in results {
        match result.map_err(std::io::Error::other)? {
            Ok(_) => published += 1,
            Err(FetchError::ArchiveCapacity { capacity: held, .. }) => {
                check!(eq; held, capacity);
                refused += 1;
            }
            Err(error) => return Err(error.into()),
        }
    }
    check!(eq; published, 1, "the shared budget admits one capture");
    check!(eq; refused, 1, "the second capture is refused, not oversubscribed");
    Ok(())
}

#[test]
fn a_retained_stage_charges_archive_capacity_so_the_cap_still_refuses() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"first capture body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let capacity_admission = ArchiveCapacity::new(needed, &archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root.path())?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    let stage = archive_stage(root.path(), "stage-first")?;
    preserve_capture(&archive, &stage, "new", body, &capture, &mut admission)?;
    drop(admission);
    drop(archive_lock_admission);
    let second_body = b"second capture body that will fail to publish";
    let second = capture_of(root.path(), second_body)?;
    let second_needed = needed_bytes(second_body, &second);
    let capacity_admission2 = ArchiveCapacity::new(needed + second_needed, &archive)?;
    let archive_lock_admission2 = ArchiveLock::acquire(root.path())?;
    let mut admission2 = ArchiveAdmission::begin_with_capacity(&archive_lock_admission2, capacity_admission2)?;
    let second_stage = archive_stage(root.path(), "stage-fail")?;
    let stage_body = second_stage.join("new.body");
    std::fs::write(&stage_body, second_body)?;
    let stage_meta = second_stage.join("new.meta.json");
    std::fs::write(&stage_meta, &second.encoded)?;
    admission2.finish_capture(
        &second.meta.content_digest,
        "0000000000000000000000000000000000000000000000000000000000000000",
    )?;
    drop(admission2);
    drop(archive_lock_admission2);
    let third_body = b"third body";
    let third = capture_of(root.path(), third_body)?;
    let capacity_admission3 = ArchiveCapacity::new(needed + second_needed, &archive)?;
    let archive_lock_admission3 = ArchiveLock::acquire(root.path())?;
    let mut admission3 = ArchiveAdmission::begin_with_capacity(&archive_lock_admission3, capacity_admission3)?;
    let third_stage = archive_stage(root.path(), "stage-third")?;
    let outcome = preserve_capture(
        &archive,
        &third_stage,
        "new",
        third_body,
        &third,
        &mut admission3,
    );
    let Err(FetchError::ArchiveCapacity { .. }) = outcome else {
        return Err(
            "a retained stage must charge capacity and refuse the next capture at the cap".into(),
        );
    };
    check!(second_stage.join("new.body").exists());
    check!(second_stage.join("new.meta.json").exists());
    Ok(())
}

#[test]
fn a_stage_under_capacity_is_still_admitted_and_published() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"under capacity body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let capacity_admission = ArchiveCapacity::new(needed * 2, &archive)?;
    let archive_lock_admission = ArchiveLock::acquire(root.path())?;
    let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
    let stage = archive_stage(root.path(), "stage-admitted")?;
    preserve_capture(&archive, &stage, "new", body, &capture, &mut admission)?;
    assert_capture(root.path(), body, &capture.meta)?;
    Ok(())
}

#[test]
fn capacity_accounting_is_exact_across_repeated_cycles_without_drift_or_double_charge() -> TestResult
{
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"repeated capture";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    for cycle in 0..3 {
        let capacity_admission = ArchiveCapacity::new(needed, &archive)?;
        let archive_lock_admission = ArchiveLock::acquire(root.path())?;
        let mut admission = ArchiveAdmission::begin_with_capacity(&archive_lock_admission, capacity_admission)?;
        let stage = archive_stage(root.path(), &format!("stage-cycle-{cycle}"))?;
        preserve_capture(&archive, &stage, "new", body, &capture, &mut admission)?;
        drop(admission);
        drop(archive_lock_admission);
    }
    let capacity_final_admission = ArchiveCapacity::new(needed, &archive)?;
    let archive_lock_final_admission = ArchiveLock::acquire(root.path())?;
    let mut final_admission = ArchiveAdmission::begin_with_capacity(&archive_lock_final_admission, capacity_final_admission)?;
    let final_stage = archive_stage(root.path(), "stage-final")?;
    let extra_body = b"extra capture";
    let extra = capture_of(root.path(), extra_body)?;
    let outcome = preserve_capture(
        &archive,
        &final_stage,
        "new",
        extra_body,
        &extra,
        &mut final_admission,
    );
    let Err(FetchError::ArchiveCapacity { .. }) = outcome else {
        return Err("accounting drifted: an extra capture should have been refused".into());
    };
    Ok(())
}
