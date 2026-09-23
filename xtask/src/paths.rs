//! Repository layout, resolved from this crate's own manifest directory.
//!
//! Resolution never depends on the caller's working directory, so every child process can be run
//! from the repository root and a relative `--store var/census-service` resolves the same way here
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

/// The `census-service` crate directory: the run crate, the composition root.
pub fn census_crate() -> PathBuf {
    repo_root().join("crates").join("census-service")
}

/// The `census-crawl` crate directory: the acquisition plane.
pub fn crawl_crate() -> PathBuf {
    repo_root().join("crates").join("census-crawl")
}

/// The acquisition plane's source root: one file — or one directory — per provider, the shared
/// kernels, the `net` layer, and the crate root that declares them (`lib.rs`).
///
/// Adapters were modules of the run crate (`src/sources/<name>`) until the crawl wave of ADR-007, so
/// an adapter is a top-level module here rather than a child of one.
pub fn adapters_dir() -> PathBuf {
    crawl_crate().join("src")
}

/// The crawl crate's captured fixtures, one directory per source. They travel with the adapters that
/// read them; the run crate's parity tests are their second consumer.
pub fn fixtures_dir() -> PathBuf {
    crawl_crate().join("tests").join("fixtures")
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
/// [`files`] with no skip list, filtered to Rust sources: the traversal rules below hold for both.
pub fn rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(files(dir, &[])?
        .into_iter()
        .filter(|path| path.extension() == Some(OsStr::new("rs")))
        .collect())
}

/// Every file under `dir`, recursively, sorted by path, staying out of the subtrees named in `skip`.
///
/// A skipped name is matched at any depth (`target`, `.git`, `var`): a nested build directory is as
/// much build output as the top-level one. Directory symlinks are not descended into and symlinked
/// files are listed, which is what `pathlib.Path.rglob("*")` did for the deleted Python scans. A
/// directory that cannot be read is an error rather than an empty list: a scan that silently measured
/// zero files would report debt as burnt down.
pub fn files(dir: &Path, skip: &[&str]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(dir, skip, &mut files)?;
    files.sort();
    Ok(files)
}

/// Depth-first collection behind [`files`], staying out of the subtrees [`skipped`] names.
fn collect_files(dir: &Path, skip: &[&str], files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("listing {}", relative(dir)))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("listing {}", relative(dir)))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .with_context(|| format!("reading the type of {}", relative(&path)))?;
        if !kind.is_dir() {
            files.push(path);
        } else if !skipped(&path, skip) {
            collect_files(&path, skip, files)?;
        }
    }
    Ok(())
}

/// Whether a directory's own name is one the walk stays out of.
fn skipped(dir: &Path, skip: &[&str]) -> bool {
    dir.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| skip.contains(&name))
}
