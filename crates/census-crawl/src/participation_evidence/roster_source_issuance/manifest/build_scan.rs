use super::{format, inputs, BuildResult};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct SourceBudget {
    entries: usize,
    bytes: u64,
}

pub(super) fn tree(
    root: &Path,
    relative: &str,
    budget: &mut SourceBudget,
) -> BuildResult<format::Tree> {
    let mut files = inventory(root, relative, budget)?;
    files.sort();
    let mut hash = Sha256::new();
    hash.update(b"roster-source-tree/v1\0");
    let bytes = files.iter().try_fold(0_u64, |sum, path| {
        let remaining = format::MAX_SOURCE_BYTES
            .checked_sub(budget.bytes)
            .ok_or("roster source byte limit")?;
        let input = inputs::file_limited(root, path, remaining)?;
        budget.bytes = budget
            .bytes
            .checked_add(input.bytes)
            .ok_or("roster source byte overflow")?;
        let encoded = serde_json::to_vec(&input)?;
        hash.update(u64::try_from(encoded.len())?.to_be_bytes());
        hash.update(encoded);
        let next = sum
            .checked_add(input.bytes)
            .ok_or("roster source byte overflow")?;
        if next > format::MAX_SOURCE_BYTES {
            return Err("roster source byte limit".into());
        }
        Ok::<_, Box<dyn std::error::Error>>(next)
    })?;
    if files.is_empty() {
        return Err("roster parser source tree empty".into());
    }
    Ok(format::Tree {
        path: relative.into(),
        sha256: format!("{:x}", hash.finalize()),
        bytes,
        files: u32::try_from(files.len())?,
    })
}

fn inventory(root: &Path, relative: &str, budget: &mut SourceBudget) -> BuildResult<Vec<String>> {
    inputs::path(relative)?;
    inputs::no_links(root, relative)?;
    let mut pending = Vec::new();
    pending.try_reserve_exact(format::MAX_FILES)?;
    pending.push(root.join(relative));
    let mut files = Vec::new();
    files.try_reserve_exact(format::MAX_FILES)?;
    for _ in 0..format::MAX_FILES {
        let Some(directory) = pending.pop() else {
            return Ok(files);
        };
        if !std::fs::symlink_metadata(&directory)?.is_dir() {
            return Err("roster source directory symlink refused".into());
        }
        inputs::watch(&directory)?;
        for entry in std::fs::read_dir(&directory)?.take(format::MAX_FILES + 1) {
            budget.entries = budget
                .entries
                .checked_add(1)
                .ok_or("roster source entry overflow")?;
            if budget.entries > format::MAX_FILES {
                return Err("roster source entry limit".into());
            }
            visit(root, entry?.path(), &mut pending, &mut files)?;
        }
    }
    Err("roster source directory limit".into())
}

fn visit(
    root: &Path,
    path: PathBuf,
    pending: &mut Vec<PathBuf>,
    files: &mut Vec<String>,
) -> BuildResult<()> {
    let metadata = std::fs::symlink_metadata(&path)?;
    let relative = path
        .strip_prefix(root)?
        .to_str()
        .ok_or("non-UTF8 roster source path")?;
    inputs::path(relative)?;
    if relative.split('/').count() > 32 {
        return Err("roster source depth limit".into());
    }
    if metadata.is_dir() {
        pending.push(path);
    } else if metadata.is_file() {
        files.push(relative.into());
    } else {
        return Err("roster source symlink or special file refused".into());
    }
    Ok(())
}
