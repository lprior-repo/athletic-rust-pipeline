use super::{BrowserError, BrowserOutcome, BrowserSettings};
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

pub(super) fn prepare_profile(settings: &BrowserSettings) -> anyhow::Result<()> {
    let path = &settings.profile_dir;
    if path.exists() {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            anyhow::bail!("browser profile must be a real directory");
        }
    } else {
        fs::create_dir_all(path)?;
        restrict_directory(path)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)?.permissions().mode();
        if mode & 0o077 != 0 {
            anyhow::bail!("browser profile directory is accessible by other users");
        }
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

pub(super) fn reject_pending(pending: &mut VecDeque<Pending>, error: BrowserError) {
    // A request that never reached the transport still leaves with a classified outcome: the
    // verdict is derived from the same cause the drain was recorded with, not written by hand.
    let outcome = BrowserOutcome::failed(error);
    pending.drain(..).for_each(|item| {
        if item.reply.send(outcome.clone()).is_err() {
            tracing::debug!("pending reply send failed");
        }
    });
}
