use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
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
    validate_checkpoint(checkpoint)?;
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
pub fn ensure_exists(output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating checkpoint directory {}", output_dir.display()))?;
    let path = output_dir.join("checkpoint.jsonl");
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error).context("creating checkpoint file"),
    }

}
fn validate_checkpoint(checkpoint: &AlphaCheckpoint) -> Result<()> {
    let fields = [
        checkpoint.key.state_code.as_str(),
        checkpoint.key.gender.as_str(),
        checkpoint.key.event_short.as_str(),
        checkpoint.key.continuation.as_str(),
        checkpoint.status.as_str(),
    ];
    if fields.iter().any(|field| {
        let lower = field.to_ascii_lowercase();
        field.contains(['\n', '\r'])
            || lower.contains("email")
            || lower.contains("phone")
            || lower.contains("street")
            || lower.contains("postal")
            || lower.contains("cookie")
            || lower.contains("authorization")
            || lower.contains("bearer ")
            || lower.contains("token")
    }) {
        bail!("checkpoint contains forbidden sensitive metadata");
    }
    Ok(())
}
fn same_unit(left: &AlphaUnitKey, right: &AlphaUnitKey) -> bool {
    left.state_code == right.state_code
        && left.season_id == right.season_id
        && left.gender == right.gender
        && left.event_short == right.event_short
}

pub fn load_latest(output_dir: &Path) -> Result<BTreeMap<AlphaUnitKey, AlphaCheckpoint>> {
    let path = output_dir.join("checkpoint.jsonl");
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = String::from_utf8(fs::read(&path).with_context(|| format!("reading {}", path.display()))?)
        .context("checkpoint is not valid UTF-8")?;
    let total_lines = text.lines().count();
    let mut latest = BTreeMap::new();
    for (line_number, segment) in text.split_inclusive('\n').enumerate() {
        let complete = segment.ends_with('\n');
        let line = segment.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            continue;
        }
        let checkpoint: AlphaCheckpoint = match serde_json::from_str(line) {
            Ok(checkpoint) => checkpoint,
            Err(_error) if !complete && line_number + 1 == total_lines => break,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("decoding checkpoint line {}", line_number + 1));
            }
        };
        let previous = latest
            .keys()
            .find(|key| same_unit(key, &checkpoint.key))
            .cloned();
        if let Some(previous) = previous {
            latest.remove(&previous);
        }
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
    #[test]
    fn latest_state_replaces_prior_continuation() {
        let directory = tempfile::tempdir().expect("tempdir");
        append(directory.path(), &checkpoint("0", false, "incomplete")).expect("append");
        append(directory.path(), &checkpoint("2", false, "incomplete")).expect("append");
        let latest = load_latest(directory.path()).expect("load");
        assert_eq!(latest.len(), 1);
        assert_eq!(latest.keys().next().map(|key| key.continuation.as_str()), Some("2"));
    }

    #[test]
    fn valid_unterminated_final_checkpoint_is_loaded() {
        let directory = tempfile::tempdir().expect("tempdir");
        append(directory.path(), &checkpoint("0", true, "complete")).expect("append");
        let path = directory.path().join("checkpoint.jsonl");
        let mut bytes = std::fs::read(&path).expect("read");
        bytes.pop();
        std::fs::write(&path, bytes).expect("write");
        let latest = load_latest(directory.path()).expect("load");
        assert_eq!(latest.len(), 1);
        assert!(!is_retryable(latest.values().next()));
    }

    #[test]
    fn checkpoint_sensitive_metadata_is_rejected() {
        let directory = tempfile::tempdir().expect("tempdir");
        assert!(append(directory.path(), &checkpoint("token-value", false, "incomplete")).is_err());
    }
}
