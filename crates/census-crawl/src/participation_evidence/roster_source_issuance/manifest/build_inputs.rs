use super::{format, BuildResult};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub(super) fn file(root: &Path, relative: &str) -> BuildResult<format::Input> {
    file_limited(root, relative, format::MAX_FILE_BYTES)
}

pub(super) fn file_limited(root: &Path, relative: &str, limit: u64) -> BuildResult<format::Input> {
    path(relative)?;
    no_links(root, relative)?;
    let full = root.join(relative);
    let metadata = std::fs::symlink_metadata(&full)?;
    if !metadata.is_file() || metadata.len() > limit.min(format::MAX_FILE_BYTES) {
        return Err("roster parser input is not a bounded regular file".into());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(usize::try_from(metadata.len())?)?;
    File::open(&full)?.take(metadata.len().checked_add(1).ok_or("roster input size overflow")?).read_to_end(&mut bytes)?;
    if bytes.len() != usize::try_from(metadata.len())? {
        return Err("roster parser input changed during read".into());
    }
    watch(&full)?;
    digest(relative, &bytes)
}

pub(super) fn digest(name: &str, bytes: &[u8]) -> BuildResult<format::Input> {
    Ok(format::Input {
        path: name.into(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(super) fn path(value: &str) -> BuildResult<()> {
    if value.is_empty() || value.len() > 512
        || !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"/._-".contains(&byte))
        || value.starts_with('/')
        || value.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("roster parser input path is not canonical".into());
    }
    Ok(())
}

pub(super) fn watch(path: impl AsRef<Path>) -> BuildResult<()> {
    let text = path.as_ref().to_str().ok_or("non-UTF8 roster parser source path")?;
    if text.contains(['\r', '\n']) {
        return Err("invalid roster parser rerun path".into());
    }
    writeln!(std::io::stdout(), "cargo:rerun-if-changed={text}")?;
    Ok(())
}

pub(super) fn environment() -> BuildResult<Vec<format::Input>> {
    let mut entries = Vec::new();
    entries.try_reserve_exact(128)?;
    for (index, (name, value)) in env::vars_os().take(8193).enumerate() {
        if index == 8192 { return Err("roster parser environment scan limit".into()); }
        let Some(name) = name.to_str() else { continue; };
        if !relevant(name) { continue; }
        if entries.len() == 128 { return Err("roster parser build environment count".into()); }
        let value = value.to_str().ok_or("non-UTF8 roster parser build environment")?;
        if name.len() > 128 || value.len() > 16 * 1024 {
            return Err("roster parser build environment size".into());
        }
        entries.push(digest(name, value.as_bytes())?);
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    for name in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
        writeln!(std::io::stdout(), "cargo:rerun-if-env-changed={name}")?;
    }
    for name in ["TARGET", "HOST", "PROFILE", "OPT_LEVEL", "DEBUG", "CARGO_ENCODED_RUSTFLAGS"] {
        if !entries.iter().any(|entry| entry.path == name) {
            return Err("roster parser required build configuration absent".into());
        }
    }
    Ok(entries)
}

fn relevant(name: &str) -> bool {
    name.starts_with("CARGO_CFG_") || name.starts_with("CARGO_FEATURE_")
        || matches!(name, "TARGET" | "HOST" | "PROFILE" | "OPT_LEVEL" | "DEBUG"
            | "RUSTFLAGS" | "CARGO_ENCODED_RUSTFLAGS" | "RUSTC_WRAPPER" | "RUSTC_WORKSPACE_WRAPPER")
}

pub(super) fn no_links(root: &Path, relative: &str) -> BuildResult<()> {
    let mut current = root.to_path_buf();
    if relative.split('/').count() > 32 { return Err("roster parser input depth".into()); }
    for part in relative.split('/') {
        current.push(part);
        if std::fs::symlink_metadata(&current)?.file_type().is_symlink() {
            return Err("roster parser input symlink refused".into());
        }
    }
    Ok(())
}
