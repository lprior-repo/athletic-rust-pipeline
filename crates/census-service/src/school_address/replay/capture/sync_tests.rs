use super::{quarantined, sidecar, temporaries, TestResult};
use crate::school_address::replay::capture::{
    atomic, io, preserve_with_sync, preserve_with_syncs, refused, ReplayError,
};
use std::cell::Cell;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn a_legacy_identical_capture_requires_a_successful_inode_sync() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"complete legacy capture")?;
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
                std::io::Error::other("injected retained capture sync failure"),
            ))
        } else {
            atomic::sync_regular(path)
        }
    };

    match preserve_with_syncs(
        &target,
        b"complete legacy capture",
        &atomic::fsync_parent,
        &sync_file,
    ) {
        Err(ReplayError::Io { path, source }) => {
            check!(eq; path, target);
            check!(source
                .to_string()
                .contains("injected retained capture sync failure"));
        }
        other => {
            return Err(format!("retained capture sync failure was not returned: {other:?}").into())
        }
    }
    check!(eq; std::fs::read(&target)?, b"complete legacy capture");
    check!(eq; calls.get(), 1);
    check!(quarantined(dir.path())?.is_empty());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());

    preserve_with_syncs(
        &target,
        b"complete legacy capture",
        &atomic::fsync_parent,
        &sync_file,
    )?;

    check!(eq; calls.get(), 2);
    check!(eq; std::fs::read(&target)?, b"complete legacy capture");
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_complete_legacy_sidecar_is_synced_before_recovery_unlinks_the_original() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"damaged")?;
    let fail_parent = |path: &Path| {
        Err(io(
            path,
            std::io::Error::other("injected setup parent sync failure"),
        ))
    };
    check!(preserve_with_sync(&target, b"correct", &fail_parent).is_err());
    let retained = quarantined(dir.path())?;
    check!(eq; retained.len(), 1);
    let note = sidecar(&retained.first().ok_or("quarantine is missing")?.0);
    let note_bytes = std::fs::read(&note)?;
    std::fs::write(&note, &note_bytes)?;
    let calls = Cell::new(0_u32);
    let sync_file = |path: &Path| {
        if path != note {
            return atomic::sync_regular(path);
        }
        let next = calls
            .get()
            .checked_add(1)
            .ok_or_else(|| refused(path, "sync counter overflow"))?;
        calls.set(next);
        if next == 1 {
            Err(io(
                path,
                std::io::Error::other("injected retained sidecar sync failure"),
            ))
        } else {
            atomic::sync_regular(path)
        }
    };

    match preserve_with_syncs(&target, b"correct", &atomic::fsync_parent, &sync_file) {
        Err(ReplayError::Io { path, source }) => {
            check!(eq; path, note);
            check!(source
                .to_string()
                .contains("injected retained sidecar sync failure"));
        }
        other => {
            return Err(format!("retained sidecar sync failure was not returned: {other:?}").into())
        }
    }
    check!(eq; calls.get(), 1);
    check!(eq; std::fs::read(&target)?, b"damaged");
    check!(eq; std::fs::read(&note)?, note_bytes);
    check!(eq; quarantined(dir.path())?, retained);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());

    preserve_with_syncs(&target, b"correct", &atomic::fsync_parent, &sync_file)?;

    check!(eq; calls.get(), 2);
    check!(eq; std::fs::read(&target)?, b"correct");
    check!(eq; std::fs::read(&note)?, note_bytes);
    check!(eq; quarantined(dir.path())?, retained);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}
