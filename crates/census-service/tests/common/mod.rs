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
