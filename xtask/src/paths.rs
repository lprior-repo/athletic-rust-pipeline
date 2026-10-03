use anyhow::Result;
use std::path::{Path, PathBuf};

#[path = "paths/walk.rs"]
mod walk;

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
    rust_files_excluding(dir, &[])
}

pub fn rust_files_excluding(dir: &Path, skip: &[&str]) -> Result<Vec<PathBuf>> {
    walk::run(dir, skip, walk::Selection::Rust)
}

pub fn files(dir: &Path, skip: &[&str]) -> Result<Vec<PathBuf>> {
    walk::run(dir, skip, walk::Selection::All)
}
