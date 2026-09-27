use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

pub fn census_crate() -> PathBuf {
    repo_root().join("crates").join("census-service")
}

pub fn crawl_crate() -> PathBuf {
    repo_root().join("crates").join("census-crawl")
}

pub fn adapters_dir() -> PathBuf {
    crawl_crate().join("src")
}

pub fn fixtures_dir() -> PathBuf {
    crawl_crate().join("tests").join("fixtures")
}

pub fn relative(path: &Path) -> String {
    path.strip_prefix(repo_root()).map_or_else(
        |_| path.display().to_string(),
        |inside| inside.display().to_string(),
    )
}

pub fn rust_files(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(files(dir, &[])?
        .into_iter()
        .filter(|path| path.extension() == Some(OsStr::new("rs")))
        .collect())
}

pub fn files(dir: &Path, skip: &[&str]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(dir, skip, &mut files)?;
    files.sort();
    Ok(files)
}

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

fn skipped(dir: &Path, skip: &[&str]) -> bool {
    dir.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| skip.contains(&name))
}
