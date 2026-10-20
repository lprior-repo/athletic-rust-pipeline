use super::super::budget::{RecoveryFsOperation as Op, TargetAccount};
use crate::acquisition::CaptureError;
use crate::{StoreError, StoreResult};
use census_domain::model::{MissingLocation, PreparedCaptureInputV1};
use std::fs::Metadata;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

pub(in crate::acquisition::recovery) struct Paths {
    pub body: PathBuf,
    pub metadata: PathBuf,
    archive: PathBuf,
    bodies: PathBuf,
    captures: PathBuf,
    content: PathBuf,
}
impl Paths {
    pub(in crate::acquisition::recovery) fn new(
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
    pub(in crate::acquisition::recovery) fn body_missing(
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
    pub(in crate::acquisition::recovery) fn metadata_missing(
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
    let parents =
        chain
            .iter()
            .take(3)
            .try_fold((), |(), (path, location)| match inspect(path, account) {
                Ok(None) => ControlFlow::Break(Ok(Some(*location))),
                Ok(Some(metadata)) if metadata.is_dir() => ControlFlow::Continue(()),
                Ok(Some(_)) => ControlFlow::Break(Err(CaptureError::PreparedRepairRequired.into())),
                Err(error) => ControlFlow::Break(Err(error)),
            });
    match parents {
        ControlFlow::Break(result) => result,
        ControlFlow::Continue(()) => match inspect(leaf, account)? {
            None => Ok(Some(absent)),
            Some(metadata) if metadata.is_file() => Ok(None),
            Some(_) => Err(CaptureError::PreparedRepairRequired.into()),
        },
    }
}

pub(in crate::acquisition::recovery) fn inspect(
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

pub(in crate::acquisition::recovery) fn safe_root(
    root: &Path,
    account: &mut TargetAccount<'_>,
) -> StoreResult<()> {
    safe_ancestry(root, account, PathPurpose::StoreRoot)
}
#[derive(Clone, Copy)]
pub(super) enum PathPurpose {
    StoreRoot,
    Candidate,
}
pub(super) fn safe_ancestry(
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
    absolute
        .components()
        .take(65)
        .enumerate()
        .try_fold(
            PathBuf::new(),
            |mut parent, (position, component)| -> StoreResult<PathBuf> {
                if position == 64 {
                    return Err(CaptureError::ReadBudgetLimitExceeded {
                        resource: "path depth",
                        required: 65,
                        limit: 64,
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
                Ok(parent)
            },
        )
        .map(|_| ())
}

#[cfg(unix)]
pub(in crate::acquisition::recovery) fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}
#[cfg(not(unix))]
pub(in crate::acquisition::recovery) fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len() && left.file_type() == right.file_type()
}

pub(super) fn io(
    path: &Path,
    source: std::io::Error,
    account: &mut TargetAccount<'_>,
) -> StoreError {
    if let Err(error) = account.diagnostic_bytes(path.as_os_str().len()) {
        return error;
    }
    let mut owned = PathBuf::new();
    if let Err(source) = owned.try_reserve_exact(path.as_os_str().len()) {
        return CaptureError::allocation_reservation_failed(
            "retained path buffer",
            Some(path.as_os_str().len()),
            source,
        )
        .into();
    }
    owned.push(path);
    StoreError::Io {
        path: owned,
        source,
    }
}
