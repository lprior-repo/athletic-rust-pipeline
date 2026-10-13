use super::capture;
use super::codec::FrozenCodecError;
use super::manifest::FrozenManifest;
use super::objects;
use super::recover;
use super::root::{self, FrozenRoot, Retained};
use super::verify;
use super::{codec_error, invalid};
use crate::report::{io_error, ReportError, ReportResult};
use census_store::{InputRecord, InputState, Store};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const ATTEMPT_TAG: &str = "freeze-v2";
pub(super) const LEASE_OWNER: &str = "census-report-freeze-v2";
pub(super) const LEASE_EXPIRY: &str = "2999-12-31";

pub enum CancelOutcome {
    Absent,
    Retained { attempt: String },
    Abandoned { attempt: String, swept: u64 },
}

pub(super) fn freeze(store: &Store, job: &str) -> ReportResult<FrozenRoot> {
    let directory = directory(store, job)?;
    mkdir(&directory)?;
    mkdir(&objects::directory(&directory))?;
    let _lock = super::capture_lock(&directory.join("lock"))?;
    #[cfg(test)]
    checkpoint("capture-lock")?;
    resume(store, job, &directory)
}

pub(super) fn cancel(store: &Store, job: &str) -> ReportResult<CancelOutcome> {
    let directory = directory(store, job)?;
    match std::fs::symlink_metadata(&directory) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CancelOutcome::Absent);
        }
        Err(source) => return Err(io_error(&directory, source)),
    }
    let _lock = super::capture_lock(&directory.join("lock"))?;
    let Some(retained) = root::read_retained(&directory)? else {
        return Ok(CancelOutcome::Absent);
    };
    recover::cancel_attempt(store, &directory, &retained)
}

pub(super) fn directory(store: &Store, job: &str) -> ReportResult<PathBuf> {
    Ok(store.out_dir().join(ATTEMPT_TAG).join(job_digest(job)?))
}

pub(super) fn stage(store: &Store, job: &str, directory: &Path) -> ReportResult<Staged> {
    let objects_dir = objects::directory(directory);
    let manifest = capture::capture(store, job, &objects_dir)?;
    let encoded = manifest.encode().map_err(codec_error)?;
    #[cfg(test)]
    checkpoint("manifest-encoded")?;
    let manifest_digest = objects::install(&objects_dir, &encoded)?;
    #[cfg(test)]
    checkpoint("manifest-installed")?;
    let inventory = verify::inventory_of(&objects_dir, &manifest_digest, &manifest)?;
    let bytes = root::payload_bytes_of(&manifest)?;
    let attempt = attempt_digest(job, &manifest)?;
    let retained = Retained {
        attempt: attempt.clone(),
        manifest: manifest_digest.clone(),
    };
    root::write_retained(directory, &retained)?;
    #[cfg(test)]
    checkpoint("root-retained")?;
    let record = InputRecord::new(&attempt, bytes, &inventory)?;
    store.put_input_record(&record)?;
    #[cfg(test)]
    checkpoint("input-requested")?;
    let (epoch, claimed) =
        store.input_lease_acquire(&attempt, record.revision(), LEASE_OWNER, LEASE_EXPIRY)?;
    #[cfg(test)]
    checkpoint("input-lease-acquired")?;
    let frozen_input = store.input_freeze(&attempt, claimed.revision, epoch, &inventory)?;
    #[cfg(test)]
    checkpoint("input-freezing")?;
    Ok(Staged {
        directory: directory.to_path_buf(),
        manifest,
        manifest_digest,
        attempt,
        epoch,
        revision: frozen_input.revision,
        bytes,
        inventory,
    })
}

#[derive(Debug, Clone)]
pub(super) struct Staged {
    pub(super) directory: PathBuf,
    pub(super) manifest: FrozenManifest,
    pub(super) manifest_digest: String,
    pub(super) attempt: String,
    pub(super) epoch: u64,
    pub(super) revision: u64,
    pub(super) bytes: u64,
    pub(super) inventory: Vec<String>,
}

pub(super) fn root_of(staged: &Staged) -> FrozenRoot {
    FrozenRoot::new(
        staged.directory.clone(),
        staged.manifest.clone(),
        staged.manifest_digest.clone(),
        staged.attempt.clone(),
        staged.bytes,
    )
}

fn resume(store: &Store, job: &str, directory: &Path) -> ReportResult<FrozenRoot> {
    match root::read_retained(directory)? {
        None => fresh(store, job, directory),
        Some(retained) => retained_root(store, job, directory, &retained),
    }
}

fn retained_root(
    store: &Store,
    job: &str,
    directory: &Path,
    retained: &Retained,
) -> ReportResult<FrozenRoot> {
    let Some(record) = store.get_input_record(&retained.attempt)? else {
        root::remove_retained(directory)?;
        return fresh(store, job, directory);
    };
    match record.state() {
        InputState::Frozen => reuse(store, job, directory, retained, &record),
        InputState::Requested | InputState::Freezing => {
            recover::recover(store, directory, retained, &record)
        }
        InputState::Abandoned => {
            root::remove_retained(directory)?;
            fresh(store, job, directory)
        }
    }
}

fn fresh(store: &Store, job: &str, directory: &Path) -> ReportResult<FrozenRoot> {
    let staged = stage(store, job, directory)
        .map_err(|error| refusal::retain(directory, "capture", error))?;
    recover::bind(store, &staged).map_err(|error| refusal::retain(directory, "bind", error))
}

fn reuse(
    store: &Store,
    job: &str,
    directory: &Path,
    retained: &Retained,
    record: &InputRecord,
) -> ReportResult<FrozenRoot> {
    match reusable(store, job, directory, retained, record) {
        Ok(root) => Ok(root),
        Err(ReuseFailure::Preserve(error)) => Err(error),
        Err(ReuseFailure::Retire(error)) => Err(recover::retire_pointer(directory, error)),
    }
}

enum ReuseFailure {
    Preserve(ReportError),
    Retire(ReportError),
}

impl From<ReportError> for ReuseFailure {
    fn from(error: ReportError) -> Self {
        Self::Retire(error)
    }
}

impl ReuseFailure {
    fn from_codec(error: FrozenCodecError) -> Self {
        match error {
            unsupported @ FrozenCodecError::UnsupportedVersion { .. } => {
                Self::Preserve(codec_error(unsupported))
            }
            other => Self::Retire(codec_error(other)),
        }
    }
}

fn reusable(
    store: &Store,
    job: &str,
    directory: &Path,
    retained: &Retained,
    record: &InputRecord,
) -> Result<FrozenRoot, ReuseFailure> {
    let objects_dir = objects::directory(directory);
    let encoded = objects::read(&objects_dir, &retained.manifest, objects::MANIFEST_CEILING)?;
    let manifest = FrozenManifest::decode(&encoded).map_err(ReuseFailure::from_codec)?;
    recover::ensure_job(&manifest, job)?;
    recover::ensure_identity(store, &manifest)?;
    verify::verify_record(&objects_dir, &retained.manifest, &manifest, record)?;
    recover::release_bound_lease(store, &retained.attempt, record)?;
    Ok(FrozenRoot::new(
        directory.to_path_buf(),
        manifest,
        retained.manifest.clone(),
        retained.attempt.clone(),
        record.bytes(),
    ))
}

fn job_digest(job: &str) -> ReportResult<String> {
    if job.is_empty() || job.len() > 4096 {
        return Err(invalid("export job identity must contain 1..=4096 bytes"));
    }
    super::digest(&job)
}

fn attempt_digest(job: &str, manifest: &FrozenManifest) -> ReportResult<String> {
    let token = manifest.input_token.digest().map_err(codec_error)?;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| invalid(&format!("reading the freeze attempt clock: {error}")))?;
    super::digest(&(ATTEMPT_TAG, job, token, std::process::id(), time.as_nanos()))
}

fn mkdir(directory: &Path) -> ReportResult<()> {
    census_store::fs::create_dir_all_synced(directory).map_err(|source| io_error(directory, source))
}

#[cfg(test)]
pub(super) fn checkpoint(name: &str) -> ReportResult<()> {
    crash_tests::checkpoint(name).map_err(|error| invalid(&error.to_string()))
}

#[cfg(test)]
#[path = "crash_tests.rs"]
mod crash_tests;

#[path = "refusal.rs"]
pub(super) mod refusal;
