#![cfg(unix)]

use census_store::{ArchiveWriteFence, CaptureError, StoreError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn given_a_held_fence_when_probed_then_contention_is_nonblocking() -> TestResult {
    let root = tempfile::tempdir()?;
    let held = ArchiveWriteFence::acquire(root.path())?;
    if ArchiveWriteFence::try_acquire_existing(root.path())?.is_some() {
        return Err("the probe acquired a fence held by another open file".into());
    }
    drop(held);
    let released = ArchiveWriteFence::try_acquire_existing(root.path())?
        .ok_or("the released fence could not be reacquired")?;
    if released.root() != root.path() {
        return Err("the reacquired fence belongs to a different archive root".into());
    }
    Ok(())
}

#[test]
fn given_a_missing_fence_when_probed_then_no_lock_is_created() -> TestResult {
    let root = tempfile::tempdir()?;
    match ArchiveWriteFence::try_acquire_existing(root.path()) {
        Err(StoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err("an absent fence must refuse with NotFound".into()),
    }
    if root.path().join("archive-capacity.lock").try_exists()? {
        return Err("the probe created a previously absent lock file".into());
    }
    Ok(())
}

#[test]
fn given_a_symlink_fence_when_probed_then_it_refuses_without_target_mutation() -> TestResult {
    let root = tempfile::tempdir()?;
    let target = tempfile::NamedTempFile::new()?;
    std::fs::write(target.path(), b"unchanged archive evidence")?;
    std::os::unix::fs::symlink(target.path(), root.path().join("archive-capacity.lock"))?;
    match ArchiveWriteFence::try_acquire_existing(root.path()) {
        Err(StoreError::Capture(CaptureError::PreparedRepairRequired)) => {}
        Err(error) => return Err(format!("unexpected symlink refusal: {error}").into()),
        Ok(_) => return Err("a symlink fence was accepted".into()),
    }
    if std::fs::read(target.path())? != b"unchanged archive evidence" {
        return Err("refusal changed the symlink target".into());
    }
    Ok(())
}
