use crate::{StoreError, StoreResult};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub struct ArchiveWriteFence {
    pub(super) file: File,
    root: PathBuf,
}

impl ArchiveWriteFence {
    pub fn acquire(http_root: &Path) -> StoreResult<Self> {
        let path = http_root.join("archive-capacity.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|source| io(&path, source))?;
        file.lock().map_err(|source| io(&path, source))?;
        Ok(Self {
            file,
            root: http_root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl crate::Store {
    pub fn matches_http_cache_root(&self, candidate: &Path) -> bool {
        matches_root(self.root(), candidate)
    }
}

fn matches_root(root: &Path, candidate: &Path) -> bool {
    root.components()
        .chain(std::iter::once(std::path::Component::Normal(
            std::ffi::OsStr::new("http"),
        )))
        .eq(candidate.components())
}

fn io(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub(super) fn preflight(
    root: &Path,
    account: &mut super::budget::TargetAccount<'_>,
) -> StoreResult<()> {
    let path = root.join("archive-capacity.lock");
    match super::backing::inspect(&path, account)? {
        None => Ok(()),
        Some(metadata) if metadata.is_file() => Ok(()),
        Some(_) => Err(crate::CaptureError::PreparedRepairRequired.into()),
    }
}
pub(super) fn verify(
    fence: &ArchiveWriteFence,
    account: &mut super::budget::TargetAccount<'_>,
) -> StoreResult<()> {
    super::backing::safe_root(fence.root(), account)?;
    let path = fence.root().join("archive-capacity.lock");
    let named = super::backing::inspect(&path, account)?
        .ok_or(crate::CaptureError::PreparedAssessmentChanged)?;
    account.direct_operation(super::RecoveryFsOperation::Inspect)?;
    let held = fence.file.metadata().map_err(|source| io(&path, source))?;
    if !named.is_file() || !held.is_file() || !super::backing::same_file(&named, &held) {
        return Err(crate::CaptureError::PreparedAssessmentChanged.into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_http_root_match_preserves_path_join_equality() {
        for root in [
            "",
            ".",
            "store",
            "store/",
            "store/.",
            "/",
            "/tmp/store",
            "/tmp/store/",
            "/tmp/./store",
            "/tmp/store/..",
        ] {
            for suffix in [
                "http",
                "http/",
                "./http",
                "http/.",
                "archive",
                "http/other",
                "HTTP",
            ] {
                let candidate = Path::new(root).join(suffix);
                assert_eq!(
                    matches_root(Path::new(root), &candidate),
                    Path::new(root).join("http").as_path() == candidate.as_path()
                );
            }
        }
    }
}
