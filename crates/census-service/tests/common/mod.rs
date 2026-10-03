use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub fn fixtures_dir() -> Result<PathBuf> {
    Ok(crawl_crate_dir()?.join("tests").join("fixtures"))
}

fn crawl_crate_dir() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("../census-crawl");
    if !dir.is_dir() {
        bail!("missing crawl crate at {}", dir.display());
    }
    Ok(dir)
}

pub fn fixture(source: &str, file: &str) -> Result<String> {
    let path = fixtures_dir()?.join(source).join(file);
    fs::read_to_string(&path).with_context(|| format!("reading fixture {}", path.display()))
}

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
