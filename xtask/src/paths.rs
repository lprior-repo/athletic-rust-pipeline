//! Repository layout, resolved from this crate's own manifest directory.
//!
//! Resolution never depends on the caller's working directory, so every child process can be run
//! from the repository root and a relative `--store var/midwest-census` resolves the same way here
//! as it does in `AGENTS.md`.

use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository root: this crate sits directly under it.
pub fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

/// The `midwest-census` crate directory.
pub fn census_crate() -> PathBuf {
    repo_root().join("crates").join("midwest-census")
}

/// The crate's source adapters (one flat module per adapter today, one directory module per adapter
/// once the decomposition lands).
pub fn sources_dir() -> PathBuf {
    census_crate().join("src").join("sources")
}

/// The crate's captured fixtures, one directory per source.
pub fn fixtures_dir() -> PathBuf {
    census_crate().join("tests").join("fixtures")
}

/// `path` relative to the repository root when it is inside it, for messages that read like the
/// paths in `git status`; the absolute path is kept when it is outside.
pub fn relative(path: &Path) -> String {
    path.strip_prefix(repo_root()).map_or_else(
        |_| path.display().to_string(),
        |inside| inside.display().to_string(),
    )
}

/// Every `.rs` file under `dir`, recursively, sorted by path.
///
/// Directory symlinks are not descended into and symlinked `.rs` files are listed, which is what
/// `pathlib.Path.rglob("*.rs")` did for the deleted Python scans. A directory that cannot be read is
/// an error rather than an empty list: a scan that silently measured zero files would report debt as
/// burnt down.
pub fn rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_rust_files(dir, &mut files)?;
    files.sort();
    Ok(files)
}

/// Depth-first collection behind [`rust_files`].
fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("listing {}", relative(dir)))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("listing {}", relative(dir)))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .with_context(|| format!("reading the type of {}", relative(&path)))?;
        if kind.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension() == Some(OsStr::new("rs")) {
            files.push(path);
        }
    }
    Ok(())
}
