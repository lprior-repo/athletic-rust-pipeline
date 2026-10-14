use super::budget::{RecoveryFsOperation as Op, RecoveryPayloadRole as Role, TargetAccount};
use crate::acquisition::{CaptureError, PreparedCandidateRejection};
use crate::{StoreError, StoreResult};
use census_domain::model::*;
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};

pub(super) struct Paths {
    pub body: PathBuf,
    pub metadata: PathBuf,
    archive: PathBuf,
    bodies: PathBuf,
    captures: PathBuf,
    content: PathBuf,
}
impl Paths {
    pub(super) fn new(
        http: &Path,
        capture: &PreparedCaptureInputV1,
        account: &mut TargetAccount<'_>,
    ) -> StoreResult<Self> {
        let length = http
            .as_os_str()
            .len()
            .checked_add(200)
            .ok_or(StoreError::CounterOverflow)?;
        account.path_bytes(length)?;
        account.reserve_owned(length)?;
        account.reserve_owned(length.checked_mul(6).ok_or(StoreError::CounterOverflow)?)?;
        let archive = http.join("archive");
        let bodies = archive.join("bodies");
        let captures = archive.join("captures");
        let content = captures.join(capture.content_digest().as_str());
        let body = bodies.join(format!("{}.body", capture.content_digest()));
        let metadata = content.join(format!("{}.meta.json", capture.metadata_digest()));
        Ok(Self {
            body,
            metadata,
            archive,
            bodies,
            captures,
            content,
        })
    }
    pub(super) fn body_missing(
        &self,
        account: &mut TargetAccount<'_>,
    ) -> StoreResult<Option<MissingLocation>> {
        missing_chain(
            &[
                (&self.archive, MissingLocation::ArchiveDirectory),
                (&self.bodies, MissingLocation::BodiesDirectory),
            ],
            &self.body,
            MissingLocation::BodyLeaf,
            account,
        )
    }
    pub(super) fn metadata_missing(
        &self,
        account: &mut TargetAccount<'_>,
    ) -> StoreResult<Option<MissingLocation>> {
        missing_chain(
            &[
                (&self.archive, MissingLocation::ArchiveDirectory),
                (&self.captures, MissingLocation::CapturesDirectory),
                (&self.content, MissingLocation::ContentCaptureDirectory),
            ],
            &self.metadata,
            MissingLocation::MetadataLeaf,
            account,
        )
    }
}

fn missing_chain(
    chain: &[(&Path, MissingLocation)],
    leaf: &Path,
    absent: MissingLocation,
    account: &mut TargetAccount<'_>,
) -> StoreResult<Option<MissingLocation>> {
    for (path, location) in chain.iter().take(3) {
        match inspect(path, account)? {
            None => return Ok(Some(*location)),
            Some(metadata) if metadata.is_dir() => {}
            Some(_) => return Err(CaptureError::PreparedRepairRequired.into()),
        }
    }
    match inspect(leaf, account)? {
        None => Ok(Some(absent)),
        Some(metadata) if metadata.is_file() => Ok(None),
        Some(_) => Err(CaptureError::PreparedRepairRequired.into()),
    }
}

pub(super) fn inspect(
    path: &Path,
    account: &mut TargetAccount<'_>,
) -> StoreResult<Option<Metadata>> {
    account.direct_operation(Op::Inspect)?;
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io(path, source, account)),
    }
}

pub(super) fn safe_root(root: &Path, account: &mut TargetAccount<'_>) -> StoreResult<()> {
    safe_ancestry(root, account, PathPurpose::StoreRoot)
}
#[derive(Clone, Copy)]
enum PathPurpose {
    StoreRoot,
    Candidate,
}
fn safe_ancestry(
    root: &Path,
    account: &mut TargetAccount<'_>,
    purpose: PathPurpose,
) -> StoreResult<()> {
    account.path_bytes(root.as_os_str().len())?;
    account.reserve_owned(
        root.as_os_str()
            .len()
            .checked_add(4096)
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|source| io(root, source, account))?
            .join(root)
    };
    let mut parent = PathBuf::new();
    for (position, component) in absolute.components().take(65).enumerate() {
        if position == 64 {
            return Err(CaptureError::ReadBudgetExceeded {
                resource: "path depth",
            }
            .into());
        }
        match component {
            std::path::Component::RootDir | std::path::Component::Normal(_) => {
                parent.push(component)
            }
            _ => return Err(CaptureError::PreparedRepairRequired.into()),
        }
        account.direct_operation(Op::Inspect)?;
        match std::fs::symlink_metadata(&parent) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => return Err(CaptureError::PreparedRepairRequired.into()),
            Err(source)
                if source.kind() == std::io::ErrorKind::NotFound
                    && matches!(purpose, PathPurpose::StoreRoot) =>
            {
                return Err(CaptureError::RequiredBackingMissing.into());
            }
            Err(source) => return Err(io(&parent, source, account)),
        }
    }
    Ok(())
}

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

#[cfg(unix)]
pub(super) fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}
#[cfg(not(unix))]
pub(super) fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len() && left.file_type() == right.file_type()
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
    let mut remaining = length;
    let mut hasher = Sha256::new();
    for _ in 0..=1024 {
        if remaining == 0 {
            break;
        }
        let count =
            usize::try_from(remaining.min(32768)).map_err(|_| StoreError::CounterOverflow)?;
        let bytes = scratch
            .get_mut(..count)
            .ok_or(CaptureError::BackingIntegrity)?;
        file.read_exact(bytes)
            .map_err(|source| io(path, source, account))?;
        hasher.update(&*bytes);
        remaining = remaining
            .checked_sub(u64::try_from(count).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
    }
    if remaining != 0 || !matches_digest(&hasher.finalize().into(), digest) {
        return Err(CaptureError::PreparedRepairRequired.into());
    }
    eof(&mut file, path, role, account)?;
    sync(&file, path, account)?;
    require_descriptor(path, &file, &opened, account)
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
fn io(path: &Path, source: std::io::Error, account: &mut TargetAccount<'_>) -> StoreError {
    if let Err(error) = account.diagnostic_bytes(path.as_os_str().len()) {
        return error;
    }
    let mut owned = PathBuf::new();
    if owned.try_reserve_exact(path.as_os_str().len()).is_err() {
        return CaptureError::ReadBudgetExceeded {
            resource: "allocation",
        }
        .into();
    }
    owned.push(path);
    StoreError::Io {
        path: owned,
        source,
    }
}
