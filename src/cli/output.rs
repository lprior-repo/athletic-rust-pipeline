//! Shared command output helpers: export destination resolution and pretty JSON on stdout.

use anyhow::{Context, Result};
use std::io::Write;

pub(super) fn destination(path: std::path::PathBuf) -> Result<std::path::PathBuf> {
    let name = path
        .file_name()
        .context("export destination must name a file")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| std::path::Path::new("."), |parent| parent);
    Ok(parent
        .canonicalize()
        .context("canonicalizing export parent")?
        .join(name))
}

pub(super) fn emit(value: &impl serde::Serialize) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
