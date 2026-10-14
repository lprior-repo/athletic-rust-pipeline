use super::budget::{RecoveryFsOperation as Op, RecoveryPayloadRole as Role, TargetAccount};
use crate::acquisition::{CaptureError, PreparedCandidateRejection};
use crate::{StoreError, StoreResult};
use census_domain::model::*;
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata};
use std::io::Read;
use std::ops::ControlFlow;
use std::path::Path;

mod path;
pub(super) use path::{inspect, safe_root, same_file, Paths};
use path::{io, safe_ancestry, PathPurpose};

fn open_exact(
    path: &Path,
    expected: u64,
    account: &mut TargetAccount<'_>,
) -> StoreResult<(File, Metadata)> {
    let before = inspect(path, account)?.ok_or(CaptureError::RequiredBackingMissing)?;
    if !before.is_file() || before.len() != expected {
        return Err(CaptureError::PreparedRepairRequired.into());
    }
    account.direct_operation(Op::Open)?;
    let file = File::open(path).map_err(|source| io(path, source, account))?;
    account.direct_operation(Op::Inspect)?;
    let opened = file
        .metadata()
        .map_err(|source| io(path, source, account))?;
    if !opened.is_file() || opened.len() != expected || !same_file(&before, &opened) {
        return Err(CaptureError::PreparedAssessmentChanged.into());
    }
    Ok((file, opened))
}

pub(super) fn materialize(
    path: &Path,
    capture: &PreparedCaptureInputV1,
    account: &mut TargetAccount<'_>,
) -> StoreResult<Vec<u8>> {
    let (mut file, opened) = open_exact(path, capture.content_bytes(), account)?;
    let length =
        usize::try_from(capture.content_bytes()).map_err(|_| StoreError::CounterOverflow)?;
    account.reserve_body(capture.content_bytes())?;
    let mut body = Vec::new();
    body.try_reserve_exact(length)
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "body allocation",
        })?;
    body.resize(length, 0);
    account.payload(Role::Body, capture.content_bytes())?;
    file.read_exact(&mut body)
        .map_err(|source| io(path, source, account))?;
    eof(&mut file, path, Role::Body, account)?;
    if !matches_digest(&Sha256::digest(&body).into(), capture.content_digest()) {
        return Err(CaptureError::PreparedRepairRequired.into());
    }
    sync(&file, path, account)?;
    require_descriptor(path, &file, &opened, account)?;
    Ok(body)
}

pub(super) fn verify(
    path: &Path,
    length: u64,
    digest: &AcquisitionDigest,
    role: Role,
    account: &mut TargetAccount<'_>,
    scratch: &mut [u8; 32768],
) -> StoreResult<Metadata> {
    let (mut file, opened) = open_exact(path, length, account)?;
    account.payload(role, length)?;
    let streamed = (0..=1024).try_fold((length, Sha256::new()), |state, _| {
        if state.0 == 0 {
            return ControlFlow::Break(Ok(state));
        }
        match read_hash_chunk(state, &mut file, path, account, scratch) {
            Ok(next) => ControlFlow::Continue(next),
            Err(error) => ControlFlow::Break(Err(error)),
        }
    });
    let (remaining, hasher) = match streamed {
        ControlFlow::Continue(state) => state,
        ControlFlow::Break(result) => result?,
    };
    if remaining != 0 || !matches_digest(&hasher.finalize().into(), digest) {
        return Err(CaptureError::PreparedRepairRequired.into());
    }
    eof(&mut file, path, role, account)?;
    sync(&file, path, account)?;
    require_descriptor(path, &file, &opened, account)
}

fn read_hash_chunk(
    (remaining, mut hasher): (u64, Sha256),
    file: &mut File,
    path: &Path,
    account: &mut TargetAccount<'_>,
    scratch: &mut [u8; 32768],
) -> StoreResult<(u64, Sha256)> {
    let count = usize::try_from(remaining.min(32768)).map_err(|_| StoreError::CounterOverflow)?;
    let bytes = scratch
        .get_mut(..count)
        .ok_or(CaptureError::BackingIntegrity)?;
    file.read_exact(bytes)
        .map_err(|source| io(path, source, account))?;
    hasher.update(&*bytes);
    let remaining = remaining
        .checked_sub(u64::try_from(count).map_err(|_| StoreError::CounterOverflow)?)
        .ok_or(StoreError::CounterOverflow)?;
    Ok((remaining, hasher))
}

fn require_descriptor(
    path: &Path,
    file: &File,
    opened: &Metadata,
    account: &mut TargetAccount<'_>,
) -> StoreResult<Metadata> {
    account.direct_operation(Op::Inspect)?;
    let after = file
        .metadata()
        .map_err(|source| io(path, source, account))?;
    let named = inspect(path, account)?.ok_or(CaptureError::PreparedAssessmentChanged)?;
    if !named.is_file()
        || after.len() != opened.len()
        || named.len() != opened.len()
        || !same_file(&named, opened)
        || !same_file(&after, opened)
        || after
            .modified()
            .map_err(|source| io(path, source, account))?
            != opened
                .modified()
                .map_err(|source| io(path, source, account))?
    {
        return Err(CaptureError::PreparedAssessmentChanged.into());
    }
    Ok(after)
}

fn eof(
    file: &mut File,
    path: &Path,
    role: Role,
    account: &mut TargetAccount<'_>,
) -> StoreResult<()> {
    account.eof_probe(role)?;
    let mut byte = [0_u8; 1];
    if file
        .read(&mut byte)
        .map_err(|source| io(path, source, account))?
        != 0
    {
        return Err(CaptureError::PreparedAssessmentChanged.into());
    }
    Ok(())
}
fn sync(file: &File, path: &Path, account: &mut TargetAccount<'_>) -> StoreResult<()> {
    account.direct_operation(Op::Sync)?;
    file.sync_all()
        .map_err(|source| io(path, source, account))?;
    let parent = path.parent().ok_or(CaptureError::BackingIntegrity)?;
    account.direct_operation(Op::Sync)?;
    crate::fs::fsync_dir(parent).map_err(|error| match error {
        StoreError::Io { path, source } => io(&path, source, account),
        error => error,
    })
}

pub(super) fn candidate(
    path: &Path,
    capture: &PreparedCaptureInputV1,
    account: &mut TargetAccount<'_>,
) -> StoreResult<Vec<u8>> {
    let metadata = inspect_candidate(path, capture, account)?;
    account.direct_operation(Op::Open)?;
    let mut file = File::open(path).map_err(|source| io(path, source, account))?;
    account.direct_operation(Op::Inspect)?;
    let opened = file
        .metadata()
        .map_err(|source| io(path, source, account))?;
    if !opened.is_file() || opened.len() != metadata.len() || !same_file(&metadata, &opened) {
        return Err(rejected(PreparedCandidateRejection::Changed));
    }
    let length =
        usize::try_from(capture.content_bytes()).map_err(|_| StoreError::CounterOverflow)?;
    account.reserve_body(capture.content_bytes())?;
    let mut body = Vec::new();
    body.try_reserve_exact(length)
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "body allocation",
        })?;
    body.resize(length, 0);
    account.payload(Role::Body, capture.content_bytes())?;
    file.read_exact(&mut body)
        .map_err(|source| io(path, source, account))?;
    eof(&mut file, path, Role::Body, account).map_err(|error| match error {
        StoreError::Capture(CaptureError::PreparedAssessmentChanged) => {
            rejected(PreparedCandidateRejection::Changed)
        }
        error => error,
    })?;
    if !matches_digest(&Sha256::digest(&body).into(), capture.content_digest()) {
        return Err(rejected(PreparedCandidateRejection::WrongDigest));
    }
    Ok(body)
}
fn inspect_candidate(
    path: &Path,
    capture: &PreparedCaptureInputV1,
    account: &mut TargetAccount<'_>,
) -> StoreResult<Metadata> {
    account.path_bytes(path.as_os_str().len())?;
    account.reserve_owned(path.as_os_str().len())?;
    let parent = path
        .parent()
        .ok_or_else(|| rejected(PreparedCandidateRejection::UnsupportedPath))?;
    safe_ancestry(parent, account, PathPurpose::Candidate).map_err(|error| match error {
        StoreError::Capture(CaptureError::PreparedRepairRequired) => {
            rejected(PreparedCandidateRejection::UnsafeParent)
        }
        error => error,
    })?;
    account.direct_operation(Op::Inspect)?;
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source, account))?;
    if !metadata.is_file() {
        return Err(rejected(PreparedCandidateRejection::NonRegular));
    }
    if metadata.len() > MAX_ACQUISITION_BODY_BYTES {
        return Err(rejected(PreparedCandidateRejection::TooLarge {
            limit: MAX_ACQUISITION_BODY_BYTES,
            observed: metadata.len(),
        }));
    }
    if metadata.len() != capture.content_bytes() {
        return Err(rejected(PreparedCandidateRejection::WrongLength {
            expected: capture.content_bytes(),
            observed: metadata.len(),
        }));
    }
    Ok(metadata)
}

fn matches_digest(bytes: &[u8; 32], expected: &AcquisitionDigest) -> bool {
    let hex = b"0123456789abcdef";
    bytes.iter().enumerate().all(|(index, byte)| {
        expected.as_str().as_bytes().get(index * 2) == hex.get(usize::from(byte >> 4))
            && expected.as_str().as_bytes().get(index * 2 + 1) == hex.get(usize::from(byte & 15))
    })
}
pub(super) fn empty_exact(capture: &PreparedCaptureInputV1) -> bool {
    capture.content_bytes() == 0
        && matches_digest(&Sha256::digest([]).into(), capture.content_digest())
}
fn rejected(reason: PreparedCandidateRejection) -> StoreError {
    CaptureError::PreparedCandidateRejected { reason }.into()
}
