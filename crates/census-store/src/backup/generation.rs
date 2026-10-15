use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use super::errors::{io_err, refused};
use super::files::sync_directories;
use crate::fs::fsync_dir;
use crate::StoreResult;

mod recover;

use recover::recover;
pub(super) fn normalize_path(to: &Path) -> PathBuf {
    if let Ok(canonical) = fs::canonicalize(to) {
        return canonical;
    }
    let text = to.to_string_lossy();
    let normalized = normalize_path_str(&text);
    PathBuf::from(normalized)
}

fn normalize_path_str(path: &str) -> String {
    let components = path.split('/').collect::<Vec<&str>>();
    let mut result = Vec::new();
    for comp in components {
        if comp == "." || comp.is_empty() {
            continue;
        }
        if comp == ".." {
            if result.is_empty() || result.last() == Some(&"..") {
                result.push("..");
            } else {
                result.pop();
            }
            continue;
        }
        result.push(comp);
    }
    if result.is_empty() {
        return "/".to_string();
    }
    if path.starts_with('/') {
        format!("/{}", result.join("/"))
    } else {
        result.join("/")
    }
}
pub(super) fn destination_prefix(base: &str, to: &Path) -> String {
    let normalized = normalize_path(to);
    let mut hasher = Sha256::new();
    let raw = normalized.as_os_str().as_encoded_bytes();
    hasher.update(raw);
    let result = hasher.finalize();
    let mut hash_hex = String::with_capacity(16);
    for byte in result.iter().take(8) {
        hash_hex.push_str(&format!("{byte:02x}"));
    }
    format!("{base}{hash_hex}.")
}

const STAGING_ATTEMPTS: usize = 4;

pub(super) struct Generation {
    path: PathBuf,
    to: PathBuf,
    published: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PromotionStep {
    Exchange,
    Promotion,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Recovered {
    pub staging_removed: u64,
    pub restored: bool,
    pub trashes_removed: u64,
}

impl Generation {
    pub(super) fn create(to: &Path, prefix: &str) -> StoreResult<Self> {
        let recovered = recover(to, prefix)?;
        let prefix = destination_prefix(prefix, to);
        if recovered.restored || recovered.staging_removed > 0 || recovered.trashes_removed > 0 {
            tracing::warn!(
                "recovered {} before staging a new generation: staging_removed={} trashes_removed={} restored={}",
                to.display(),
                recovered.staging_removed,
                recovered.trashes_removed,
                recovered.restored
            );
        }
        let parent = parent_of(to);
        for _ in 0..STAGING_ATTEMPTS {
            let candidate = parent.join(format!("{prefix}{}", unique_token()));
            match fs::create_dir(&candidate) {
                Ok(()) => {
                    return Ok(Self {
                        path: candidate,
                        to: to.to_path_buf(),
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

    pub(super) fn publish(self) -> StoreResult<()> {
        self.publish_with(&mut promote)
    }

    pub(super) fn publish_with(
        mut self,
        rename: &mut impl FnMut(PromotionStep, &Path, &Path) -> io::Result<()>,
    ) -> StoreResult<()> {
        sync_directories(&self.path)?;
        let step = match fs::symlink_metadata(&self.to) {
            Ok(_) => PromotionStep::Exchange,
            Err(error) if error.kind() == io::ErrorKind::NotFound => PromotionStep::Promotion,
            Err(source) => return Err(io_err(&self.to, source)),
        };
        rename(step, &self.path, &self.to).map_err(|source| io_err(&self.to, source))?;
        fsync_dir(parent_of(&self.to))?;
        if step == PromotionStep::Exchange {
            discard(&self.path);
        }
        self.published = true;
        Ok(())
    }
}

pub(super) fn promote(step: PromotionStep, from: &Path, to: &Path) -> io::Result<()> {
    match step {
        PromotionStep::Promotion => fs::rename(from, to),
        PromotionStep::Exchange => exchange(from, to),
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn exchange(from: &Path, to: &Path) -> io::Result<()> {
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        from,
        rustix::fs::CWD,
        to,
        rustix::fs::RenameFlags::EXCHANGE,
    )
    .map_err(io::Error::from)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn exchange(_from: &Path, _to: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic directory exchange is required to replace a backup generation",
    ))
}

fn discard(path: &Path) {
    if let Err(error) = fs::remove_dir_all(path) {
        if error.kind() != io::ErrorKind::NotFound {
            tracing::warn!("could not remove {}: {error}", path.display());
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

#[cfg(test)]
#[path = "generation_tests.rs"]
mod generation_tests;
