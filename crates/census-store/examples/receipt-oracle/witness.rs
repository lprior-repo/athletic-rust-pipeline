use crate::args::Fault;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const WITNESS_EXCLUDE: &str = "fjall/lock";

pub fn walk(root: &Path) -> Result<BTreeMap<String, (u64, String)>, Fault> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| Fault::io(format!("reading {}: {error}", directory.display())))?;
        for entry in entries {
            let entry = entry.map_err(|error| Fault::io(format!("walking store: {error}")))?;
            let path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|error| Fault::io(format!("stat {}: {error}", path.display())))?;
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|error| Fault::io(error.to_string()))?
                .to_string_lossy()
                .to_string();
            let digest = sha256_file(&path)?;
            files.insert(relative, (metadata.len(), digest));
        }
    }
    Ok(files)
}

fn sha256_file(path: &Path) -> Result<String, Fault> {
    let bytes = fs::read(path)
        .map_err(|error| Fault::io(format!("reading {}: {error}", path.display())))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

pub fn witness_json(files: &BTreeMap<String, (u64, String)>) -> serde_json::Value {
    let entries = files
        .iter()
        .map(|(path, (len, digest))| serde_json::json!({ "path": path, "length": len, "sha256": digest }))
        .collect::<Vec<_>>();
    serde_json::json!(entries)
}

pub fn witness_diff(
    before: &BTreeMap<String, (u64, String)>,
    after: &BTreeMap<String, (u64, String)>,
) -> Vec<String> {
    let mut changed = Vec::new();
    for (path, value) in before {
        if path == WITNESS_EXCLUDE {
            continue;
        }
        if after.get(path) != Some(value) {
            changed.push(path.clone());
        }
    }
    for path in after.keys() {
        if path == WITNESS_EXCLUDE {
            continue;
        }
        if !before.contains_key(path) {
            changed.push(path.clone());
        }
    }
    changed.sort();
    changed
}
