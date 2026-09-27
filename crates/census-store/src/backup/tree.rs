use std::fs;
use std::io;
use std::path::Path;

use super::errors::{io_err, object_kind, refused};
use super::files::copy_file;
use crate::StoreResult;

pub(super) const STORE_ROOTS: [&str; 5] = ["fjall", "entities", "journal", "http", "out"];

pub(super) const LOCK_FILE: &str = "lock";

pub(super) const DB_DIR: &str = "fjall";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Copied {
    pub files: u64,
    pub bytes: u64,
}

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

fn copy_dir(src: &Path, dst: &Path, copied: &mut Copied) -> StoreResult<()> {
    fs::create_dir_all(dst).map_err(|source| io_err(dst, source))?;
    let entries = fs::read_dir(src).map_err(|source| io_err(src, source))?;
    for entry in entries {
        let entry = entry.map_err(|source| io_err(src, source))?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
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
