//! The refusals and I/O failures a backup or a restore reports, each naming what it refused.
//!
//! A refusal is not a guess the caller can work around: every function below hands back the path, or
//! the object kind, that did not meet the condition, so the operator reads *what* was refused and not
//! only that something was.

use std::fs;
use std::io;
use std::path::Path;

use crate::store::StoreError;

/// A refusal: the request did not meet a condition this module will not guess around.
pub(super) fn refused(detail: impl Into<String>) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

/// An I/O failure against a named path.
pub(super) fn io_err(path: &Path, source: io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// What a filesystem object is, for a refusal message that says what was refused.
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

/// The non-regular, non-directory kinds a unix filesystem can hand a walker.
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
