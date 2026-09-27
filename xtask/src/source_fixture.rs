use crate::paths;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

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
