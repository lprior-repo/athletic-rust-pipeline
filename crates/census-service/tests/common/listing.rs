use std::fs;
use std::path::{Path, PathBuf};

use crate::common::fixtures_dir;
use anyhow::{bail, Context, Result};

pub fn fixtures(source: &str) -> Result<Vec<PathBuf>> {
    let dir = fixtures_dir()?.join(source);
    let entries =
        fs::read_dir(&dir).with_context(|| format!("listing fixtures in {}", dir.display()))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading an entry of {}", dir.display()))?
            .path();
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        bail!("no fixtures under {}", dir.display());
    }
    Ok(paths)
}

pub fn file_name(path: &Path) -> Result<String> {
    let name = path.file_name().context("fixture path has no file name")?;
    Ok(name.to_string_lossy().into_owned())
}
