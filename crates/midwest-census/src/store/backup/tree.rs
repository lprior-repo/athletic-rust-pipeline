//! The store tree walk: which directories a backup carries, and the objects it refuses to carry.
//!
//! The walk is where a backup decides what a store *is* on disk, so it is also where the refusal
//! lives: every root and every entry is stat'd with `symlink_metadata`, and an object that is not a
//! real directory or a regular file is refused by path rather than followed, read, or skipped.

use std::fs;
use std::io;
use std::path::Path;

use super::errors::{io_err, object_kind, refused};
use super::files::copy_file;
use crate::store::StoreResult;

/// The store root's durable material, in the order a backup carries it: the database, the pre-Fjall
/// logs it was imported from, the resume journal, the response cache and the exported artifacts.
pub(super) const STORE_ROOTS: [&str; 5] = ["fjall", "entities", "journal", "http", "out"];

/// fjall's advisory lock file, inside the database directory (`fjall-3.1.10/src/file.rs:11`).
pub(super) const LOCK_FILE: &str = "lock";

/// The database directory inside a store root.
pub(super) const DB_DIR: &str = "fjall";

/// What a tree walk copied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Copied {
    /// Regular files copied.
    pub files: u64,
    /// Bytes written.
    pub bytes: u64,
}

/// Copy the store's durable roots from `from` into the fresh directory `dst`.
///
/// Only regular files and real directories are copied. Every other object - a symlink, a fifo, a
/// socket, a device node - is **refused by path** rather than followed or read: a backup copies what
/// the store itself holds, and a symlink in the tree would either copy something outside it or send a
/// walker somewhere that never terminates.
pub(super) fn copy_tree(from: &Path, dst: &Path) -> StoreResult<Copied> {
    let mut copied = Copied::default();
    for root in STORE_ROOTS {
        let src = from.join(root);
        let kind = match fs::symlink_metadata(&src) {
            Ok(meta) => meta.file_type(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(source) => return Err(io_err(&src, source)),
        };
        if !kind.is_dir() {
            return Err(refused(format!(
                "backup refuses {}: it is {}, and a store root carries only directories here",
                src.display(),
                object_kind(kind)
            )));
        }
        copy_dir(&src, &dst.join(root), &mut copied)?;
    }
    Ok(copied)
}

/// Copy one directory tree, one regular file at a time.
fn copy_dir(src: &Path, dst: &Path, copied: &mut Copied) -> StoreResult<()> {
    fs::create_dir_all(dst).map_err(|source| io_err(dst, source))?;
    let entries = fs::read_dir(src).map_err(|source| io_err(src, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| io_err(src, source))?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        // `symlink_metadata` is the point: `metadata` would follow a symlink and report what it points
        // at, which is how a walker leaves the tree it was asked to copy.
        let kind = fs::symlink_metadata(&path)
            .map_err(|source| io_err(&path, source))?
            .file_type();
        if kind.is_dir() {
            copy_dir(&path, &target, copied)?;
        } else if kind.is_file() {
            let streamed = copy_file(&path, &target)?;
            copied.files = copied.files.saturating_add(1);
            copied.bytes = copied.bytes.saturating_add(streamed.bytes);
        } else {
            return Err(refused(format!(
                "backup refuses {}: it is {}, and a backup copies regular files and directories only",
                path.display(),
                object_kind(kind)
            )));
        }
    }
    Ok(())
}
