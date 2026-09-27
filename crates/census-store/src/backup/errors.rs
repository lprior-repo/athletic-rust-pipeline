use std::fs;
use std::io;
use std::path::Path;

use crate::StoreError;

pub(super) fn refused(detail: impl Into<String>) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

pub(super) fn io_err(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub(super) fn object_kind(kind: fs::FileType) -> &'static str {
    if kind.is_symlink() {
        "a symlink"
    } else if kind.is_dir() {
        "a directory"
    } else if kind.is_file() {
        "a regular file"
    } else {
        special_kind(kind)
    }
}

#[cfg(unix)]
fn special_kind(kind: fs::FileType) -> &'static str {
    use std::os::unix::fs::FileTypeExt;
    if kind.is_fifo() {
        "a fifo"
    } else if kind.is_socket() {
        "a socket"
    } else if kind.is_block_device() {
        "a block device"
    } else if kind.is_char_device() {
        "a character device"
    } else {
        "not a regular file or a directory"
    }
}

#[cfg(not(unix))]
fn special_kind(_kind: fs::FileType) -> &'static str {
    "not a regular file or a directory"
}
