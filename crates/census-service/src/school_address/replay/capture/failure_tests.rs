use super::{quarantined, sidecar, temporaries, TestResult};
use crate::school_address::replay::capture::{
    atomic, hex, identical, io, preserve, preserve_with_sync, publish, quarantine, refused,
    ReplayError,
};
use std::cell::Cell;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn a_partial_legacy_sidecar_is_retained_and_does_not_block_recovery() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("manifest.json");
    std::fs::write(&target, b"damaged")?;
    let old_evidence = target.with_extension(format!("recovered-{}", hex(b"damaged")));
    std::fs::write(&old_evidence, b"damaged")?;
    let partial_sidecar = sidecar(&old_evidence);
    std::fs::write(&partial_sidecar, b"partial")?;

    preserve(&target, b"correct")?;

    check!(eq; std::fs::read(&target)?, b"correct".to_vec());
    check!(eq; std::fs::read(&partial_sidecar)?, b"partial".to_vec());
    let evidence = quarantined(dir.path())?;
    check!(eq; evidence.len(), 2);
    let new_evidence = evidence
        .iter()
        .find(|(path, _)| path != &old_evidence)
        .ok_or("recovery did not retain a new evidence identity")?;
    let text = std::fs::read_to_string(sidecar(&new_evidence.0))?;
    check!(text.contains(&hex(b"damaged")));
    check!(text.contains(&hex(b"correct")));
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn identical_retry_repeats_the_failed_parent_sync() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let calls = Cell::new(0_u32);
    let sync = |path: &Path| {
        let next = calls.get().checked_add(1).ok_or("sync counter overflow");
        let next = next.map_err(|detail| refused(path, detail))?;
        calls.set(next);
        if next == 1 {
            Err(io(
                path,
                std::io::Error::other("injected parent sync failure"),
            ))
        } else {
            atomic::fsync_parent(path)
        }
    };

    match preserve_with_sync(&target, b"same bytes", &sync) {
        Err(ReplayError::Io { source, .. }) => {
            check!(source.to_string().contains("injected parent sync failure"));
        }
        other => return Err(format!("first sync failure was not returned: {other:?}").into()),
    }
    check!(eq; std::fs::read(&target)?, b"same bytes".to_vec());
    preserve_with_sync(&target, b"same bytes", &sync)?;
    check!(eq; calls.get(), 2);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_failed_quarantine_sync_retries_without_overwriting_evidence() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"damaged")?;
    let fail = |path: &Path| Err(io(path, std::io::Error::other("injected quarantine sync")));
    check!(preserve_with_sync(&target, b"correct", &fail).is_err());
    check!(eq; std::fs::read(&target)?, b"damaged".to_vec());
    let retained = quarantined(dir.path())?;
    check!(eq; retained.len(), 1);
    let first = retained.first().ok_or("quarantine is missing")?;
    let note = sidecar(&first.0);
    let note_before = std::fs::read(&note)?;

    preserve(&target, b"correct")?;

    check!(eq; std::fs::read(&target)?, b"correct".to_vec());
    check!(eq; std::fs::read(&note)?, note_before);
    check!(eq; quarantined(dir.path())?, retained);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_failed_evidence_file_sync_retains_the_original_and_is_repeated_on_retry() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"damaged")?;
    let calls = Cell::new(0_u32);
    let sync_file = |path: &Path| {
        let next = calls
            .get()
            .checked_add(1)
            .ok_or_else(|| refused(path, "sync counter overflow"))?;
        calls.set(next);
        if next == 1 {
            Err(io(
                path,
                std::io::Error::other("injected evidence file sync failure"),
            ))
        } else {
            atomic::sync_regular(path)
        }
    };
    let recover = || {
        atomic::with_staged(&target, b"correct", |tmp| {
            quarantine::replace_with_evidence_sync(
                &target,
                tmp,
                b"correct",
                &atomic::fsync_parent,
                &sync_file,
            )
        })?;
        atomic::fsync_parent(&target)
    };

    match recover() {
        Err(ReplayError::Io { source, .. }) => {
            check!(source
                .to_string()
                .contains("injected evidence file sync failure"));
        }
        other => return Err(format!("evidence sync failure was not returned: {other:?}").into()),
    }
    check!(eq; calls.get(), 1);
    check!(eq; std::fs::read(&target)?, b"damaged".to_vec());
    let retained = quarantined(dir.path())?;
    check!(eq; retained.len(), 1);
    let first = retained.first().ok_or("quarantine is missing")?;
    check!(!sidecar(&first.0).exists());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());

    recover()?;

    check!(eq; calls.get(), 2);
    check!(eq; std::fs::read(&target)?, b"correct".to_vec());
    check!(eq; quarantined(dir.path())?, retained);
    check!(sidecar(&first.0).is_file());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_partial_stage_failure_removes_only_the_created_temporary() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let foreign = dir.path().join("capture.tmp.foreign");
    std::fs::write(&foreign, b"other owner")?;
    let result: Result<(), ReplayError> = atomic::with_temp(&target, |tmp, file| {
        file.write_all(b"partial")
            .map_err(|source| io(tmp, source))?;
        Err(io(tmp, std::io::Error::other("injected stage failure")))
    });
    check!(result.is_err());
    check!(!target.exists());
    check!(eq; std::fs::read(&foreign)?, b"other owner".to_vec());
    check!(eq; temporaries(dir.path())?, vec!["capture.tmp.foreign".to_string()]);
    Ok(())
}

#[test]
fn a_readback_failure_removes_the_created_temporary() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let result = atomic::with_temp(&target, |tmp, file| {
        file.write_all(b"bytes").map_err(|source| io(tmp, source))?;
        std::fs::remove_file(tmp).map_err(|source| io(tmp, source))?;
        identical(tmp, b"bytes")
    });
    check!(matches!(result, Err(ReplayError::Io { .. })));
    check!(!target.exists());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_cleanup_failure_keeps_both_typed_errors() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let result: Result<(), ReplayError> = atomic::with_temp(&target, |tmp, _| {
        std::fs::remove_file(tmp).map_err(|source| io(tmp, source))?;
        std::fs::create_dir(tmp).map_err(|source| io(tmp, source))?;
        Err(refused(&target, "injected operation failure"))
    });
    match result {
        Err(ReplayError::Cleanup {
            path,
            primary,
            source,
        }) => {
            check!(path.is_dir());
            check!(matches!(*primary, ReplayError::Refused { .. }));
            check!(eq; source.kind(), std::io::ErrorKind::IsADirectory);
        }
        other => return Err(format!("cleanup failure was not preserved: {other:?}").into()),
    }
    check!(!target.exists());
    Ok(())
}

#[test]
fn an_install_failure_cleans_the_stage_and_preserves_the_non_file_occupant() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::create_dir(&target)?;
    check!(preserve(&target, b"bytes").is_err());
    check!(target.is_dir());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_report_refuses_a_symlink_without_modifying_either_destination() -> TestResult {
    let dir = TempDir::new()?;
    let original = dir.path().join("original.json");
    let target = dir.path().join("replay-evidence.json");
    std::fs::write(&original, b"original")?;
    std::os::unix::fs::symlink(&original, &target)?;
    check!(matches!(
        publish(&target, b"new"),
        Err(ReplayError::Refused { .. })
    ));
    check!(std::fs::symlink_metadata(&target)?.file_type().is_symlink());
    check!(eq; std::fs::read(&original)?, b"original".to_vec());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[cfg(unix)]
#[test]
fn replay_uses_the_report_publisher_instead_of_following_a_symlink() -> TestResult {
    let dir = TempDir::new()?;
    let out = dir.path().join("corpus");
    std::fs::create_dir(&out)?;
    let original = dir.path().join("original.json");
    let target = out.join("replay-evidence.json");
    std::fs::write(&original, b"original")?;
    std::os::unix::fs::symlink(&original, &target)?;
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/school_address_replay/manifest.json");

    match crate::school_address::replay::run(&fixture, &out) {
        Err(ReplayError::Refused { path, detail }) => {
            check!(eq; path, target);
            check!(eq; detail, "publication destination is not a regular file");
        }
        Err(other) => return Err(format!("wrong publication error: {other}").into()),
        Ok(_) => return Err("replay followed a report symlink".into()),
    }

    check!(std::fs::symlink_metadata(&target)?.file_type().is_symlink());
    check!(eq; std::fs::read(&original)?, b"original".to_vec());
    check!(eq; temporaries(&out)?, Vec::<String>::new());
    Ok(())
}
