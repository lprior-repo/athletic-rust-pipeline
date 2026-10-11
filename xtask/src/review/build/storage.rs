use super::STORE_DIRS;
use crate::cmd::Cmd;
use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;

pub(super) fn store_bytes(run_dir: &Path) -> Result<Map<String, Value>> {
    let mut out = Map::new();
    for name in STORE_DIRS {
        let path = run_dir.join(name);
        let bytes = if path.exists() { dir_bytes(&path)? } else { 0 };
        out.insert(name.to_string(), json!(bytes));
    }
    Ok(out)
}

fn dir_bytes(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    let entries = fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading entry under {}", path.display()))?;
        let metadata = entry
            .metadata()
            .with_context(|| format!("stat {}", entry.path().display()))?;
        if metadata.is_dir() {
            total = total.saturating_add(dir_bytes(&entry.path())?);
        } else {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

pub(super) fn git(args: &[&str]) -> Result<String> {
    Cmd::new("git")
        .args(args.iter().map(|arg| (*arg).to_string()))
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
}

pub(super) fn write_json(path: &Path, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{text}\n")).with_context(|| format!("writing {}", path.display()))
}

pub(super) fn relative(path: &Path) -> String {
    let root = crate::paths::repo_root();
    path.strip_prefix(&root).map_or_else(
        |_| path.display().to_string(),
        |rel| rel.display().to_string(),
    )
}
