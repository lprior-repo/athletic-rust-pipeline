use std::fs;
use std::io;
use std::path::{Component, Path};

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
    pub links: u64,
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
        copy_dir(from, &src, &dst.join(root), &mut copied)?;
    }
    Ok(copied)
}

fn copy_dir(root: &Path, src: &Path, dst: &Path, copied: &mut Copied) -> StoreResult<()> {
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
            copy_dir(root, &path, &target, copied)?;
        } else if kind.is_file() {
            let streamed = copy_file(&path, &target)?;
            copied.files = copied.files.saturating_add(1);
            copied.bytes = copied.bytes.saturating_add(streamed.bytes);
        } else if kind.is_symlink() {
            copy_relative_link(root, &path, &target, copied)?;
        } else {
            return Err(refused(format!(
                "backup refuses {}: it is {}, and a backup copies regular files, directories and \
                 store-relative links only",
                path.display(),
                object_kind(kind)
            )));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_relative_link(
    root: &Path,
    link: &Path,
    target: &Path,
    copied: &mut Copied,
) -> StoreResult<()> {
    let destination = fs::read_link(link).map_err(|source| io_err(link, source))?;
    if !store_relative_link(relative_parent(root, link), &destination) {
        return Err(link_refusal(link, &destination));
    }
    std::os::unix::fs::symlink(&destination, target).map_err(|source| io_err(target, source))?;
    copied.links = copied.links.saturating_add(1);
    Ok(())
}

pub(super) fn link_refusal(link: &Path, target: &Path) -> crate::StoreError {
    refused(format!(
        "backup refuses {}: it is a symlink to {}, and a backup copies the links that stay inside \
         the store only",
        link.display(),
        target.display()
    ))
}

#[cfg(not(unix))]
fn copy_relative_link(
    _root: &Path,
    link: &Path,
    _target: &Path,
    _copied: &mut Copied,
) -> StoreResult<()> {
    Err(refused(format!(
        "backup refuses {}: it is a symbolic link, and this host cannot copy one",
        link.display()
    )))
}

pub(super) fn relative_parent<'a>(root: &Path, path: &'a Path) -> &'a Path {
    match path.strip_prefix(root) {
        Ok(relative) => match relative.parent() {
            Some(parent) => parent,
            None => Path::new(""),
        },
        Err(_) => Path::new(""),
    }
}

pub(super) fn store_relative_link(parent: &Path, target: &Path) -> bool {
    if target.is_absolute() || target.as_os_str().is_empty() {
        return false;
    }
    let mut depth = 0_usize;
    descend(&mut depth, parent) && descend(&mut depth, target)
}

fn descend(depth: &mut usize, path: &Path) -> bool {
    for component in path.components() {
        match component {
            Component::Normal(_) => *depth = depth.saturating_add(1),
            Component::CurDir => {}
            Component::ParentDir => match depth.checked_sub(1) {
                Some(remaining) => *depth = remaining,
                None => return false,
            },
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}
