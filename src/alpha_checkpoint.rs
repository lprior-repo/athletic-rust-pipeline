use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AlphaUnitKey {
    pub state_code: String,
    pub season_id: i32,
    pub gender: String,
    pub event_short: String,
    pub continuation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlphaCheckpoint {
    pub key: AlphaUnitKey,
    pub response_count: usize,
    pub complete: bool,
    pub status: String,
}

pub fn append(output_dir: &Path, checkpoint: &AlphaCheckpoint) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating checkpoint directory {}", output_dir.display()))?;
    let path = output_dir.join("checkpoint.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening checkpoint file {}", path.display()))?;
    serde_json::to_writer(&mut file, checkpoint).context("serializing checkpoint")?;
    file.write_all(b"\n").context("writing checkpoint newline")?;
    file.sync_data().context("syncing checkpoint")?;
    Ok(())
}

pub fn load_latest(output_dir: &Path) -> Result<BTreeMap<AlphaUnitKey, AlphaCheckpoint>> {
    let path = output_dir.join("checkpoint.jsonl");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let file = File::open(&path).with_context(|| format!("opening {}", path.display()))?;
    let mut latest = BTreeMap::new();
    for (line_number, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("reading checkpoint line {}", line_number + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let checkpoint: AlphaCheckpoint = serde_json::from_str(&line)
            .with_context(|| format!("decoding checkpoint line {}", line_number + 1))?;
        latest.insert(checkpoint.key.clone(), checkpoint);
    }
    Ok(latest)
}

#[allow(dead_code)]
pub fn is_retryable(checkpoint: Option<&AlphaCheckpoint>) -> bool {
    checkpoint.map_or(true, |state| !state.complete)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkpoint(continuation: &str, complete: bool, status: &str) -> AlphaCheckpoint {
        AlphaCheckpoint {
            key: AlphaUnitKey {
                state_code: "AZ".to_owned(),
                season_id: 2026,
                gender: "m".to_owned(),
                event_short: "100m".to_owned(),
                continuation: continuation.to_owned(),
            },
            response_count: 100,
            complete,
            status: status.to_owned(),
        }
    }

    #[test]
    fn incomplete_checkpoint_is_retryable() {
        let directory = tempfile::tempdir().expect("tempdir");
        append(directory.path(), &checkpoint("0", false, "incomplete")).expect("append");
        let latest = load_latest(directory.path()).expect("load");
        assert!(is_retryable(latest.get(&checkpoint("0", false, "incomplete").key)));
    }

    #[test]
    fn latest_checkpoint_replaces_older_state() {
        let directory = tempfile::tempdir().expect("tempdir");
        append(directory.path(), &checkpoint("0", false, "incomplete")).expect("append");
        append(directory.path(), &checkpoint("0", true, "complete")).expect("append");
        let latest = load_latest(directory.path()).expect("load");
        assert_eq!(latest.len(), 1);
        assert!(!is_retryable(latest.values().next()));
        assert_eq!(latest.values().next().map(|v| v.status.as_str()), Some("complete"));
    }

    #[test]
    fn missing_checkpoint_is_retryable() {
        let directory = tempfile::tempdir().expect("tempdir");
        assert!(is_retryable(None));
        assert!(load_latest(directory.path()).expect("load").is_empty());
    }
}
