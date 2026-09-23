//! The staging directory a backup or a restore is built in, and the single rename that publishes it.
//!
//! Publication is a rename, so a generation is either wholly there or not there at all: a run that
//! fails leaves the destination exactly as it found it, including when the destination holds the only
//! good backup, which is moved aside in one rename and deleted only after the new one lands.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::errors::{io_err, refused};
use super::files::fsync_dir;
use crate::store::StoreResult;

/// How many times a staging directory name is tried before giving up. A collision needs another
/// process to have created the same name in the same nanosecond, so one retry is already generous.
const STAGING_ATTEMPTS: usize = 4;

/// A directory a generation is built in, beside the destination it will be renamed onto.
///
/// Nothing at the destination changes until [`Generation::publish`] renames the finished directory over
/// it in one step. A generation that is dropped before that is removed, so a failed backup or restore
/// leaves the destination exactly as it found it - including the case the destination already holds the
/// only good backup, which is never rewritten file by file.
pub(super) struct Generation {
    path: PathBuf,
    to: PathBuf,
    prefix: String,
    published: bool,
}

impl Generation {
    /// Create a fresh staging directory beside `to`, named `<prefix><token>`.
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

    /// Where the generation is being built.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Rename the finished generation onto the destination, replacing a previous one whole.
    ///
    /// A generation that is already there is moved aside in one rename first and deleted last, so the
    /// window in which the destination holds no generation is a single rename wide, and the previous
    /// generation is never edited in place. If the publish itself fails, the generation that was there
    /// is put back.
    pub(super) fn publish(mut self) -> StoreResult<()> {
        fsync_dir(&self.path)?;
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

    /// Move whatever the destination holds now out of the way; it is deleted only after the publish.
    fn move_superseded_aside(&self) -> StoreResult<Option<PathBuf>> {
        match fs::symlink_metadata(&self.to) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io_err(&self.to, source)),
            Ok(_) => {
                let trash = parent_of(&self.to).join(format!("{}old.{}", self.prefix, unique_token()));
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

/// The directory a sibling of `path` belongs in.
pub(super) fn parent_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// A token that does not repeat within a process, and is very unlikely to repeat across two.
fn unique_token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let count = NEXT.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("{}.{nanos}.{count}", std::process::id())
}
