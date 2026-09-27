use super::{BrowserError, BrowserOutcome, BrowserSettings};
use crate::lifecycle::error::BrowserStartupError;
use chromiumoxide::Page;
use std::{collections::VecDeque, fs, path::Path};
use tokio::sync::oneshot;

pub(super) struct PageSlot {
    pub page: Page,
    pub busy: bool,
}

impl PageSlot {
    pub fn new(page: Page) -> Self {
        Self { page, busy: false }
    }
}

pub(super) struct Pending {
    pub request: crate::request::RequestSpec,
    pub reply: oneshot::Sender<super::BrowserOutcome>,
}

pub(super) fn prepare_profile(settings: &BrowserSettings) -> Result<(), BrowserStartupError> {
    let path = &settings.profile_dir;
    if path.exists() {
        let metadata =
            fs::symlink_metadata(path).map_err(|_| BrowserStartupError::ProfileNotDirectory)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(BrowserStartupError::ProfileNotDirectory);
        }
    } else {
        fs::create_dir_all(path).map_err(|_| BrowserStartupError::ProfileNotDirectory)?;
        restrict_directory(path)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)
            .map_err(|_| BrowserStartupError::ProfileNotDirectory)?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(BrowserStartupError::ProfilePermissions);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> Result<(), BrowserStartupError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|_| BrowserStartupError::ProfileNotDirectory)?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).map_err(|_| BrowserStartupError::ProfilePermissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> Result<(), BrowserStartupError> {
    Ok(())
}

pub(super) fn reject_pending(pending: &mut VecDeque<Pending>, error: BrowserError) {
    let outcome = BrowserOutcome::failed(error);
    pending.drain(..).for_each(|item| {
        if item.reply.send(outcome.clone()).is_err() {
            tracing::debug!("pending reply send failed");
        }
    });
}

#[cfg(test)]
#[path = "pool_tests.rs"]
mod pool_tests;
