use super::super::archive::{initialize_archive, ArchiveLock};
use super::super::archive_lock_test::{self, LockEvent};
use super::*;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Operation {
    Replay,
    Write,
}

fn ordered_actor(operation: Operation) -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"retained");
    write_cache(&body_path, &meta_path, b"retained", &meta)?;
    let archive_lock = ArchiveLock::acquire(root.path())?;
    let archive_path = root.path().join("archive-capacity.lock");
    archive_lock_test::require_held(&archive_path)?;
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker_body = body_path.clone();
    let worker_meta = meta_path.clone();
    let worker = std::thread::spawn(move || -> TestResult {
        let eligible = worker_meta.with_extension("json.lock");
        let mut acquisitions = 0_u8;
        let _observation = archive_lock_test::observe(move |path, event| {
            if event == LockEvent::BeforeAcquire {
                if acquisitions != 0 {
                    return Err(archive_io::invalid(
                        path,
                        "recursive archive lock acquisition",
                    ));
                }
                acquisitions = 1;
                archive_lock_test::require_available(&eligible)?;
                ready_tx
                    .send(())
                    .map_err(|error| archive_io::invalid(path, &error.to_string()))?;
                release_rx
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|error| {
                        archive_io::invalid(
                            path,
                            &format!("actor B awaiting archive-holder A release: {error}"),
                        )
                    })?;
            }
            if event == LockEvent::Acquired {
                archive_lock_test::require_held(path)?;
            }
            Ok(())
        })?;
        match operation {
            Operation::Replay => {
                check!(eq; replay_cache(&worker_body, &worker_meta, &meta)?, Some(b"retained".to_vec()));
            }
            Operation::Write => {
                write_cache(
                    &worker_body,
                    &worker_meta,
                    b"replacement",
                    &metadata(b"replacement"),
                )?;
            }
        }
        Ok(())
    });
    let observed = ready_rx.recv_timeout(Duration::from_secs(5));
    let eligible = archive_io::lock_cache(&meta_path);
    let eligible_result = eligible.map(drop);
    drop(archive_lock);
    let released = release_tx.send(());
    let joined = worker.join().map_err(|_| "archive actor B panicked")?;
    joined?;
    observed.map_err(|error| format!("actor A awaiting B before archive acquisition: {error}"))?;
    eligible_result?;
    released?;
    match operation {
        Operation::Replay => assert_served(root.path(), b"retained", &metadata(b"retained"))?,
        Operation::Write => assert_served(root.path(), b"replacement", &metadata(b"replacement"))?,
    }
    assert_capture(root.path(), b"retained", &metadata(b"retained"))?;
    archive_lock_test::require_available(&archive_path)?;
    Ok(())
}

#[test]
fn archive_lock_order_legacy_replay_and_writer_do_not_reverse_resources() -> TestResult {
    [Operation::Replay, Operation::Write]
        .into_iter()
        .try_for_each(ordered_actor)
}

#[test]
fn archive_lock_order_busy_cache_preserves_incoming_archive_and_exact_error() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let old = metadata(b"old");
    let new = metadata(b"new");
    write_cache(&body_path, &meta_path, b"old", &old)?;
    let encoded = std::fs::read(&meta_path)?;
    let old_archive = std::fs::read(archive_meta(root.path(), &old)?)?;
    let eligible = archive_io::lock_cache(&meta_path)?;
    let result = write_cache(&body_path, &meta_path, b"new", &new);
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path == meta_path.with_extension("json.lock") && source.kind() == ErrorKind::WouldBlock));
    check!(eq; std::fs::read(&body_path)?, b"old");
    check!(eq; std::fs::read(&meta_path)?, encoded);
    check!(eq; std::fs::read(archive_meta(root.path(), &old)?)?, old_archive);
    assert_capture(root.path(), b"old", &old)?;
    assert_capture(root.path(), b"new", &new)?;
    archive_lock_test::require_available(&root.path().join("archive-capacity.lock"))?;
    drop(eligible);
    Ok(())
}

#[test]
fn archive_lock_order_lock_open_refusal_precedes_busy_eligible() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let lock_path = root.path().join("archive-capacity.lock");
    std::fs::create_dir(&lock_path)?;
    let eligible = archive_io::lock_cache(&meta_path)?;
    let outcomes = [
        write_cache(&body_path, &meta_path, b"raw", &metadata(b"raw")),
        replay_cache(&body_path, &meta_path, &metadata(b"raw")).map(|_| ()),
        with_archive_eligible_fence(&body_path, &meta_path, |_| Ok(())),
    ];
    for result in outcomes {
        check!(matches!(result, Err(FetchError::Cache { path, source })
            if path == lock_path && source.kind() == ErrorKind::IsADirectory));
    }
    check!(lock_path.is_dir());
    check!(!root.path().join("archive").exists());
    drop(eligible);
    Ok(())
}

#[test]
fn archive_lock_order_admission_refusal_precedes_busy_writer_but_replay_defers() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    write_cache(&body_path, &meta_path, b"raw", &meta)?;
    let encoded = std::fs::read(&meta_path)?;
    let record = root.path().join("archive/reservations").join(format!(
        "{}-{}",
        "a".repeat(64),
        "b".repeat(64)
    ));
    std::fs::write(&record, b"malformed")?;
    let eligible = archive_io::lock_cache(&meta_path)?;
    let writer = write_cache(&body_path, &meta_path, b"next", &metadata(b"next"));
    check!(matches!(writer, Err(FetchError::Cache { path, source })
        if path == record && source.kind() == ErrorKind::InvalidData));
    let replay = replay_cache(&body_path, &meta_path, &meta);
    check!(matches!(replay, Err(FetchError::Cache { path, source })
        if path == meta_path.with_extension("json.lock") && source.kind() == ErrorKind::WouldBlock));
    check!(eq; std::fs::read(&record)?, b"malformed");
    check!(eq; std::fs::read(&meta_path)?, encoded);
    check!(eq; std::fs::read(&body_path)?, b"raw");
    drop(eligible);
    let replay = replay_cache(&body_path, &meta_path, &meta);
    check!(matches!(replay, Err(FetchError::Cache { path, source })
        if path == record && source.kind() == ErrorKind::InvalidData));
    archive_lock_test::require_available(&root.path().join("archive-capacity.lock"))?;
    Ok(())
}

#[test]
fn archive_lock_order_replay_miss_and_mismatch_defer_archive_initialization() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let meta = metadata(b"raw");
    check!(eq; replay_cache(&body_path, &meta_path, &meta)?, None);
    check!(!root.path().join("archive").exists());
    check!(root.path().join("archive-capacity.lock").is_file());
    let archive = initialize_archive(root.path())?;
    let obstruction = archive.join("reservations");
    std::fs::write(&obstruction, b"occupied reservation directory")?;
    std::fs::write(&body_path, b"raw")?;
    std::fs::write(&meta_path, serde_json::to_vec(&meta)?)?;
    let mut expected = meta;
    expected.fetched_at = "2026-09-28T12:34:56Z".into();
    check!(eq; replay_cache(&body_path, &meta_path, &expected)?, None);
    check!(eq; std::fs::read(&obstruction)?, b"occupied reservation directory");
    check!(eq; file_names(&archive.join("captures"))?.len(), 0);
    Ok(())
}

#[test]
fn archive_lock_order_callback_errors_release_both_actual_guards() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let result: Result<(), FetchError> =
        with_archive_eligible_fence(&body_path, &meta_path, |_| {
            archive_lock_test::require_held(&root.path().join("archive-capacity.lock"))?;
            archive_lock_test::require_held(&meta_path.with_extension("json.lock"))?;
            Err(archive_io::invalid(&body_path, "injected callback refusal"))
        });
    check!(matches!(result, Err(FetchError::Cache { path, source })
        if path == body_path && source.kind() == ErrorKind::InvalidData));
    archive_lock_test::require_available(&root.path().join("archive-capacity.lock"))?;
    archive_lock_test::require_available(&meta_path.with_extension("json.lock"))?;
    check!(!root.path().join("archive").exists());
    Ok(())
}

#[test]
fn archive_lock_order_callback_unwind_releases_both_actual_guards() -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let result = std::panic::catch_unwind(|| {
        with_archive_eligible_fence(&body_path, &meta_path, |_| -> Result<(), FetchError> {
            archive_lock_test::require_held(&root.path().join("archive-capacity.lock"))?;
            archive_lock_test::require_held(&meta_path.with_extension("json.lock"))?;
            std::panic::resume_unwind(Box::new("owned callback unwind"))
        })
    });
    match result {
        Err(payload) => check!(eq; payload.downcast_ref::<&str>(), Some(&"owned callback unwind")),
        Ok(result) => return Err(format!("expected owned callback unwind, got {result:?}").into()),
    }
    archive_lock_test::require_available(&root.path().join("archive-capacity.lock"))?;
    archive_lock_test::require_available(&meta_path.with_extension("json.lock"))?;
    Ok(())
}

#[test]
fn archive_lock_order_existing_symlink_root_alias_keeps_legacy_behavior() -> TestResult {
    let directory = tempfile::tempdir()?;
    let target = directory.path().join("target");
    std::fs::create_dir(&target)?;
    let alias = directory.path().join("alias");
    std::os::unix::fs::symlink(&target, &alias)?;
    let (body_path, meta_path) = cache_paths(&alias);
    let meta = metadata(b"aliased root");
    write_cache(&body_path, &meta_path, b"aliased root", &meta)?;
    check!(eq; replay_cache(&body_path, &meta_path, &meta)?, Some(b"aliased root".to_vec()));
    assert_capture(&target, b"aliased root", &meta)?;
    check!(std::fs::symlink_metadata(&alias)?.file_type().is_symlink());
    archive_lock_test::require_available(&target.join("archive-capacity.lock"))?;
    Ok(())
}
