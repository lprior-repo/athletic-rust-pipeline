//! `source-fixture`: what is captured for one source.
//!
//! This command only reads: it lists the fixture files under
//! `crates/midwest-census/tests/fixtures/<source>/`, so a test author can see the coverage a source
//! already has before adding a case, and so a missing source is refused instead of silently
//! reported as empty.

use crate::paths;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Print every fixture file of `source`, one per line, sorted, with its byte size.
pub fn list(source: &str) -> Result<()> {
    let dir = paths::fixtures_dir().join(source);
    if !dir.is_dir() {
        return absent(source, &dir);
    }
    let files = files_under(&dir)?;
    if files.is_empty() {
        bail!(
            "no fixture files under {}: the directory exists but holds none",
            paths::relative(&dir)
        );
    }
    for file in &files {
        let bytes = fs::metadata(file)
            .with_context(|| format!("reading the size of {}", paths::relative(file)))?
            .len();
        println!("{bytes:>9} bytes  {}", paths::relative(file));
    }
    println!(
        "\n{} fixture file(s) under {}",
        files.len(),
        paths::relative(&dir)
    );
    Ok(())
}

/// Why the source has no fixture directory, and which ones do exist.
///
/// [`crate::replay`] refuses an absent source with this same message, so `source-fixture` and
/// `replay` name the directories that exist identically.
pub fn absent(source: &str, dir: &Path) -> Result<()> {
    let mut known = fixture_sources();
    known.sort();
    let hint = if known.is_empty() {
        "no source has a fixture directory yet".to_string()
    } else {
        format!("fixture directories that exist: {}", known.join(", "))
    };
    if dir.is_file() {
        bail!(
            "{} is a file, not a fixture directory ({hint})",
            paths::relative(dir)
        )
    }
    bail!(
        "no fixture directory for source `{source}` at {} ({hint})",
        paths::relative(dir)
    )
}

/// Every fixture directory under the fixture root.
fn fixture_sources() -> Vec<String> {
    let Ok(entries) = fs::read_dir(paths::fixtures_dir()) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect()
}

/// Every regular file under `dir`, recursively, sorted by path.
///
/// [`crate::replay`] walks a fixture directory with this same function, so the two commands sort and
/// filter the corpus identically.
pub fn files_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries =
            fs::read_dir(&next).with_context(|| format!("reading {}", paths::relative(&next)))?;
        for entry in entries {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}
