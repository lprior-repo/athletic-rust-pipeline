use super::freeze::{root_of, CancelOutcome, Staged, LEASE_EXPIRY, LEASE_OWNER};
use super::manifest::FrozenManifest;
use super::objects;
use super::root::{self, FrozenRoot, Retained};
use super::verify;
use super::{codec_error, invalid};
use crate::report::{ReportError, ReportResult};
use census_store::{InputRecord, InputState, Store, StoreError};
use std::path::Path;

pub(super) fn bind(store: &Store, staged: &Staged) -> ReportResult<FrozenRoot> {
    match complete(store, staged) {
        Ok(()) => Ok(root_of(staged)),
        Err(error) => Err(retire(store, staged, error)),
    }
}

pub(super) fn recover(
    store: &Store,
    job: &str,
    directory: &Path,
    retained: &Retained,
    record: &InputRecord,
) -> ReportResult<FrozenRoot> {
    let objects_dir = objects::directory(directory);
    let manifest = open_manifest(&objects_dir, &retained.manifest)?;
    ensure_job(&manifest, job)?;
    if let Err(error) = verify::verify_record(&objects_dir, &retained.manifest, &manifest, record) {
        if error.preserves_retained() {
            return Err(error.into());
        }
        return Err(abandoned(store, directory, retained, record, error.into()));
    }
    let Some((epoch, claimed_revision)) = lease_of(store, &retained.attempt, record)? else {
        return Err(retire_pointer(
            directory,
            invalid("an interrupted freeze left a lease this attempt cannot read"),
        ));
    };
    let inventory = record.chunks().to_vec();
    let revision = match record.state() {
        InputState::Requested => {
            store
                .input_freeze(&retained.attempt, claimed_revision, epoch, &inventory)?
                .revision
        }
        InputState::Freezing => claimed_revision,
        InputState::Frozen | InputState::Abandoned => {
            return Err(retire_pointer(
                directory,
                invalid("a completed freeze attempt cannot be recovered"),
            ));
        }
    };
    let staged = Staged {
        directory: directory.to_path_buf(),
        manifest,
        manifest_digest: retained.manifest.clone(),
        attempt: retained.attempt.clone(),
        epoch,
        revision,
        bytes: record.bytes(),
        inventory,
    };
    bind(store, &staged)
}

pub(super) fn cancel_attempt(
    store: &Store,
    directory: &Path,
    retained: &Retained,
) -> ReportResult<CancelOutcome> {
    let Some(record) = store.get_input_record(&retained.attempt)? else {
        root::remove_retained(directory)?;
        return Ok(CancelOutcome::Absent);
    };
    if record.state() == InputState::Frozen {
        return Ok(CancelOutcome::Retained {
            attempt: retained.attempt.clone(),
        });
    }
    if record.state() != InputState::Abandoned {
        abandon(store, &retained.attempt, &record)?;
    }
    root::remove_retained(directory)?;
    let swept = objects::sweep(&objects::directory(directory))?;
    Ok(CancelOutcome::Abandoned {
        attempt: retained.attempt.clone(),
        swept,
    })
}

pub(super) fn open_manifest(objects_dir: &Path, digest: &str) -> ReportResult<FrozenManifest> {
    let encoded = objects::read(objects_dir, digest, objects::MANIFEST_CEILING)?;
    FrozenManifest::decode(&encoded).map_err(codec_error)
}

pub(super) fn ensure_job(manifest: &FrozenManifest, job: &str) -> ReportResult<()> {
    if manifest.lineage.export_job.as_deref() != Some(job) {
        return Err(invalid(
            "a retained frozen root belongs to a different logical job",
        ));
    }
    Ok(())
}

pub(super) fn ensure_identity(store: &Store, manifest: &FrozenManifest) -> ReportResult<()> {
    if manifest.lineage.store_identity != super::store_identity(store)? {
        return Err(invalid(
            "a retained frozen root belongs to a different census store",
        ));
    }
    Ok(())
}

pub(super) fn retire_pointer(directory: &Path, error: ReportError) -> ReportError {
    if error.resource().is_some() {
        return error;
    }
    match root::remove_retained(directory) {
        Ok(()) => error,
        Err(cleanup) => ReportError::Cleanup {
            operation: Box::new(error),
            cleanup: Box::new(cleanup),
        },
    }
}

fn complete(store: &Store, staged: &Staged) -> ReportResult<()> {
    let objects_dir = objects::directory(&staged.directory);
    let verified = verify::verify_root(&objects_dir, &staged.manifest_digest, &staged.manifest)?;
    if verified != staged.inventory {
        return Err(invalid(
            "the frozen root inventory changed between capture and verification",
        ));
    }
    #[cfg(test)]
    super::freeze::checkpoint("root-verified")?;
    store.input_frozen(
        &staged.attempt,
        staged.revision,
        staged.epoch,
        staged.bytes,
        &staged.inventory,
    )?;
    #[cfg(test)]
    super::freeze::checkpoint("input-frozen")?;
    store.lease_release(&staged.attempt, staged.epoch)?;
    #[cfg(test)]
    super::freeze::checkpoint("lease-released")?;
    Ok(())
}

fn retire(store: &Store, staged: &Staged, error: ReportError) -> ReportError {
    if error.resource().is_some() {
        return error;
    }
    match retire_attempt(store, staged) {
        Ok(()) => error,
        Err(cleanup) => ReportError::Cleanup {
            operation: Box::new(error),
            cleanup: Box::new(cleanup),
        },
    }
}

fn retire_attempt(store: &Store, staged: &Staged) -> ReportResult<()> {
    match store.get_input_record(&staged.attempt)? {
        Some(record) if record.state() != InputState::Frozen => {
            store.input_abandon(&staged.attempt, record.revision(), staged.epoch)?;
            store.lease_release(&staged.attempt, staged.epoch)?;
        }
        _ => {}
    }
    root::remove_retained(&staged.directory)
}

pub(super) fn release_bound_lease(
    store: &Store,
    attempt: &str,
    record: &InputRecord,
) -> ReportResult<()> {
    let Some(epoch) = record.lease_epoch() else {
        return Ok(());
    };
    match store.lease_release(attempt, epoch) {
        Ok(()) | Err(StoreError::LeaseMismatch { actual: None, .. }) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn lease_of(
    store: &Store,
    attempt: &str,
    record: &InputRecord,
) -> ReportResult<Option<(u64, u64)>> {
    release_bound_lease(store, attempt, record)?;
    match store.input_lease_acquire(attempt, record.revision(), LEASE_OWNER, LEASE_EXPIRY) {
        Ok((epoch, claimed)) => Ok(Some((epoch, claimed.revision))),
        Err(StoreError::LeaseMismatch { .. }) => Ok(None),
        Err(failure) => Err(failure.into()),
    }
}

fn abandon(store: &Store, attempt: &str, record: &InputRecord) -> ReportResult<()> {
    let Some((epoch, revision)) = lease_of(store, attempt, record)? else {
        return Ok(());
    };
    store.input_abandon(attempt, revision, epoch)?;
    store.lease_release(attempt, epoch)?;
    Ok(())
}

fn abandoned(
    store: &Store,
    directory: &Path,
    retained: &Retained,
    record: &InputRecord,
    error: ReportError,
) -> ReportError {
    if error.resource().is_some() {
        return error;
    }
    let cleanup =
        abandon(store, &retained.attempt, record).and_then(|()| root::remove_retained(directory));
    match cleanup {
        Ok(()) => error,
        Err(cleanup) => ReportError::Cleanup {
            operation: Box::new(error),
            cleanup: Box::new(cleanup),
        },
    }
}
