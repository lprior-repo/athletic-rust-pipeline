use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::errors::{io_err, refused};
use super::files::sync_directories;
use crate::fs::fsync_dir;
use crate::StoreResult;

const STAGING_ATTEMPTS: usize = 4;

pub(super) struct Generation {
    path: PathBuf,
    to: PathBuf,
    prefix: String,
    published: bool,
}

impl Generation {
    pub(super) fn create(to: &Path, prefix: &str) -> StoreResult<Self> {
        let parent = parent_of(to);
        for _ in 0..STAGING_ATTEMPTS {
            let candidate = parent.join(format!("{prefix}{}", unique_token()));
            match fs::create_dir(&candidate) {
                Ok(()) => {
                    return Ok(Self {
                        path: candidate,
                        to: to.to_path_buf(),
                        prefix: prefix.to_string(),
                        published: false,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(io_err(&candidate, source)),
            }
        }
        Err(refused(format!(
            "could not create a staging directory beside {}: {STAGING_ATTEMPTS} names were taken",
            to.display()
        )))
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn publish(mut self) -> StoreResult<()> {
        sync_directories(&self.path)?;
        let superseded = self.move_superseded_aside()?;
        if let Err(source) = fs::rename(&self.path, &self.to) {
            if let Some(trash) = &superseded {
                if let Err(error) = fs::rename(trash, &self.to) {
                    tracing::error!(
                        "could not put {} back at {}: {error}; the superseded generation is intact there",
                        trash.display(),
                        self.to.display()
                    );
                }
            }
            return Err(io_err(&self.to, source));
        }
        self.published = true;
        fsync_dir(parent_of(&self.to))?;
        if let Some(trash) = superseded {
            if let Err(error) = fs::remove_dir_all(&trash) {
                tracing::warn!(
                    "published {}, but the superseded generation at {} could not be removed: {error}",
                    self.to.display(),
                    trash.display()
                );
            }
        }
        Ok(())
    }

    fn move_superseded_aside(&self) -> StoreResult<Option<PathBuf>> {
        match fs::symlink_metadata(&self.to) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io_err(&self.to, source)),
            Ok(_) => {
                let trash =
                    parent_of(&self.to).join(format!("{}old.{}", self.prefix, unique_token()));
                fs::rename(&self.to, &trash).map_err(|source| io_err(&self.to, source))?;
                Ok(Some(trash))
            }
        }
    }
}

impl Drop for Generation {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        match fs::remove_dir_all(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(
                "could not remove the abandoned staging directory {}: {error}",
                self.path.display()
            ),
        }
    }
}

pub(super) fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn unique_token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let count = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("{}.{nanos}.{count}", std::process::id())
}
