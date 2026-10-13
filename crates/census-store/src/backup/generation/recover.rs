use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset};

use super::{discard, parent_of, Recovered};
use crate::backup::errors::io_err;
use crate::backup::MANIFEST_PATH;
use crate::fs::fsync_dir;
use crate::{StoreError, StoreResult};

pub(super) fn recover(to: &Path, prefix: &str) -> StoreResult<Recovered> {
    let prefix = super::destination_prefix(prefix, to);
    let parent = parent_of(to);
    let listing = match fs::read_dir(parent) {
        Ok(listing) => listing,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Recovered::default()),
        Err(source) => return Err(io_err(parent, source)),
    };
    let trash_prefix = format!("{prefix}old.");
    let (staging, trashes) = collect_generations(parent, listing, &prefix, &trash_prefix)?;
    let mut recovered = Recovered::default();
    recovered = clear_staging(staging, recovered)?;
    if trashes.is_empty() {
        return Ok(recovered);
    }
    if fs::symlink_metadata(to).is_ok() {
        if to.join(MANIFEST_PATH).is_file() {
            recovered = clear_trashes(trashes, recovered)?;
        } else {
            recovered = handle_partial_trashes(to, trashes, recovered)?;
        }
        return Ok(recovered);
    }
    recovered = restore_from_trashes(to, trashes, recovered)?;
    Ok(recovered)
}

fn collect_generations(
    parent: &Path,
    listing: fs::ReadDir,
    prefix: &str,
    trash_prefix: &str,
) -> StoreResult<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut staging = Vec::new();
    let mut trashes = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|source| io_err(parent, source))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(trash_prefix) {
            trashes.push(entry.path());
        } else if name.starts_with(prefix) {
            staging.push(entry.path());
        }
    }
    Ok((staging, trashes))
}

fn clear_staging(staging: Vec<PathBuf>, recovered: Recovered) -> StoreResult<Recovered> {
    let mut result = recovered;
    for path in staging {
        discard(&path);
        result.staging_removed = count(result.staging_removed)?;
    }
    Ok(result)
}

fn clear_trashes(trashes: Vec<PathBuf>, recovered: Recovered) -> StoreResult<Recovered> {
    let mut result = recovered;
    for path in trashes {
        discard(&path);
        result.trashes_removed = count(result.trashes_removed)?;
    }
    Ok(result)
}

fn handle_partial_trashes(
    to: &Path,
    trashes: Vec<PathBuf>,
    recovered: Recovered,
) -> StoreResult<Recovered> {
    let (empty, kept): (Vec<PathBuf>, Vec<PathBuf>) =
        trashes.into_iter().partition(|path| is_empty_dir(path));
    let mut result = recovered;
    for path in empty {
        discard(&path);
        result.trashes_removed = count(result.trashes_removed)?;
    }
    if !kept.is_empty() {
        tracing::warn!(
            "{} holds no published generation and {} displaced generation(s) beside it are \
             kept for an operator: clear that path before publishing into it again",
            to.display(),
            kept.len()
        );
    }
    Ok(result)
}

fn restore_from_trashes(
    to: &Path,
    mut trashes: Vec<PathBuf>,
    recovered: Recovered,
) -> StoreResult<Recovered> {
    trashes.sort_by(|left, right| {
        written_at(right)
            .cmp(&written_at(left))
            .then_with(|| right.file_name().cmp(&left.file_name()))
    });
    let Some(source) = trashes.first().cloned() else {
        return Ok(recovered);
    };
    let parent = parent_of(to);
    fs::rename(&source, to).map_err(|error| io_err(&source, error))?;
    let mut result = recovered;
    result.restored = true;
    for path in trashes {
        if path == source {
            continue;
        }
        discard(&path);
        result.trashes_removed = count(result.trashes_removed)?;
    }
    fsync_dir(parent)?;
    Ok(result)
}

fn written_at(generation: &Path) -> Option<DateTime<FixedOffset>> {
    let text = fs::read_to_string(generation.join(MANIFEST_PATH)).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&text).ok()?;
    let recorded = manifest.get("written_at")?.as_str()?;
    DateTime::parse_from_rfc3339(recorded).ok()
}

fn is_empty_dir(path: &Path) -> bool {
    match fs::read_dir(path) {
        Ok(mut listing) => listing.next().is_none(),
        Err(_) => false,
    }
}

fn count(value: u64) -> StoreResult<u64> {
    value.checked_add(1).ok_or(StoreError::CounterOverflow)
}
