use super::{quarantined, sidecar, temporaries, TestResult};
use crate::school_address::replay::capture::{
    atomic, hex, io, preserve, preserve_with_syncs, refused, ReplayError,
};
use std::cell::Cell;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::TempDir;

const LINK_CHANGE: &str = "capture changed while evidence was linked";

#[test]
fn a_recovery_that_finds_its_expected_capture_already_reinstalled_reuses_it() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"damaged")?;
    let calls = Cell::new(0_u32);
    let reused = Cell::new(false);
    let sync = |path: &Path| {
        let next = calls
            .get()
            .checked_add(1)
            .ok_or_else(|| refused(path, "sync counter overflow"))?;
        calls.set(next);
        if next == 2 {
            std::fs::write(path, b"correct").map_err(|source| io(path, source))?;
        }
        atomic::fsync_parent(path)
    };
    let sync_file = |path: &Path| {
        if path == target.as_path() {
            reused.set(true);
        }
        atomic::sync_regular(path)
    };

    preserve_with_syncs(&target, b"correct", &sync, &sync_file)?;

    check!(
        reused.get(),
        "the recovery did not reuse the reinstalled object"
    );
    check!(eq; std::fs::read(&target)?, b"correct".to_vec());
    let retained = quarantined(dir.path())?;
    check!(eq; retained.len(), 1);
    check!(eq; retained[0].1, b"damaged".to_vec());
    check!(sidecar(&retained[0].0).is_file());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_recovery_that_finds_a_different_capture_reinstalled_is_refused_without_overwriting(
) -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"damaged")?;
    let calls = Cell::new(0_u32);
    let sync = |path: &Path| {
        let next = calls
            .get()
            .checked_add(1)
            .ok_or_else(|| refused(path, "sync counter overflow"))?;
        calls.set(next);
        if next == 2 {
            std::fs::write(path, b"interloper").map_err(|source| io(path, source))?;
        }
        atomic::fsync_parent(path)
    };

    match preserve_with_syncs(&target, b"correct", &sync, &atomic::sync_regular) {
        Err(ReplayError::Refused { path, detail }) => {
            check!(eq; path, target);
            check!(eq; detail, "capture destination changed during recovery");
        }
        other => {
            return Err(
                format!("a reinstalled different capture was not refused: {other:?}").into(),
            )
        }
    }
    check!(eq; std::fs::read(&target)?, b"interloper".to_vec());
    let retained = quarantined(dir.path())?;
    check!(eq; retained.len(), 1);
    check!(eq; retained[0].1, b"damaged".to_vec());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_capture_that_changes_after_evidence_linking_is_refused_before_replacement() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"damaged")?;
    let calls = Cell::new(0_u32);
    let sync = |path: &Path| {
        let next = calls
            .get()
            .checked_add(1)
            .ok_or_else(|| refused(path, "sync counter overflow"))?;
        calls.set(next);
        if next == 1 {
            std::fs::remove_file(path).map_err(|source| io(path, source))?;
            std::fs::write(path, b"hostile replacement").map_err(|source| io(path, source))?;
        }
        atomic::fsync_parent(path)
    };

    match preserve_with_syncs(&target, b"correct", &sync, &atomic::sync_regular) {
        Err(ReplayError::Refused { path, detail }) => {
            check!(eq; path, target);
            check!(eq; detail, "capture changed during quarantine");
        }
        other => {
            return Err(
                format!("a capture changed during quarantine was not refused: {other:?}").into(),
            )
        }
    }
    check!(eq; std::fs::read(&target)?, b"hostile replacement".to_vec());
    let retained = quarantined(dir.path())?;
    check!(eq; retained.len(), 1);
    check!(eq; retained[0].1, b"damaged".to_vec());
    check!(sidecar(&retained[0].0).is_file());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());

    preserve(&target, b"correct")?;

    check!(eq; std::fs::read(&target)?, b"correct".to_vec());
    check!(eq; quarantined(dir.path())?.len(), 2);
    Ok(())
}

#[test]
fn a_capture_that_changes_while_evidence_is_linked_is_refused() -> TestResult {
    for _ in 0..3 {
        if evidence_link_race_attempt()? {
            return Ok(());
        }
    }
    Err("the linked-evidence refusal was not reached within three race attempts".into())
}

fn evidence_link_race_attempt() -> TestResult<bool> {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    let observed = vec![0xB4_u8; 24 * 1024 * 1024];
    std::fs::write(&target, &observed)?;
    let quarantine = target.with_extension(format!("recovered-{}", hex(&observed)));
    let window = 8 * 1024;
    let starts = [observed.len() - window, observed.len() / 2];
    std::thread::scope(|scope| -> TestResult<bool> {
        let racers = starts.map(|start| {
            let quarantine = quarantine.clone();
            scope.spawn(move || overwrite_linked_evidence(&quarantine, start, window))
        });
        let result = preserve_with_syncs(
            &target,
            b"correct",
            &atomic::fsync_parent,
            &atomic::sync_regular,
        );
        for racer in racers {
            racer
                .join()
                .map_err(|_| "a quarantine race thread panicked")?
                .map_err(|error| error.to_string())?;
        }
        match result {
            Err(ReplayError::Refused { path, detail }) if detail == LINK_CHANGE => {
                check!(eq; path, quarantine);
                check!(target.is_file());
                check!(quarantine.is_file());
                check!(eq; temporaries(dir.path())?, Vec::<String>::new());
                Ok(true)
            }
            Err(ReplayError::Refused { .. }) | Ok(()) => Ok(false),
            Err(other) => Err(format!("the changed capture was not refused: {other:?}").into()),
        }
    })
}

fn overwrite_linked_evidence(
    quarantine: &Path,
    start: usize,
    window: usize,
) -> Result<(), std::io::Error> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::fs::symlink_metadata(quarantine).is_err() {
        if std::time::Instant::now() > deadline {
            return Err(std::io::Error::other(
                "the quarantine evidence link never appeared",
            ));
        }
        std::hint::spin_loop();
    }
    let offset = u64::try_from(start)
        .map_err(|_| std::io::Error::other("the overwrite offset is not representable"))?;
    let mut file = std::fs::OpenOptions::new().write(true).open(quarantine)?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(&vec![0xC3_u8; window])?;
    Ok(())
}
