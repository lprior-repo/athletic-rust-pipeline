use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
};

const MAX_RECORD_BYTES: u64 = 16 * 1024 * 1024;

struct BoundedBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedBuffer {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl std::io::Write for BoundedBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let required = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("JSONL buffer length overflow"))?;
        if required > self.limit {
            return Err(std::io::Error::other("JSONL record exceeds 16 MiB"));
        }
        if required > self.bytes.capacity() {
            let doubled = self.bytes.capacity().saturating_mul(2);
            let target = doubled.max(required).min(self.limit);
            let additional = target
                .checked_sub(self.bytes.len())
                .ok_or_else(|| std::io::Error::other("JSONL buffer capacity invariant violated"))?;
            self.bytes
                .try_reserve_exact(additional)
                .map_err(|_| std::io::Error::other("unable to reserve JSONL record buffer"))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn serialize_record<T: Serialize>(record: &T) -> Result<Vec<u8>> {
    let limit = usize::try_from(MAX_RECORD_BYTES)?
        .checked_sub(1)
        .context("JSONL record limit underflow")?;
    let mut output = BoundedBuffer::new(limit);
    serde_json::to_writer(&mut output, record).context("serializing committed record")?;
    let mut bytes = output.into_bytes();
    bytes
        .try_reserve_exact(1)
        .context("reserving JSONL commit marker")?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// A newline is the commit marker. Recover only an interrupted final append;
/// malformed committed records fail closed. Caller must own the run lock.
pub fn load<T: DeserializeOwned>(
    path: &Path,
    key: impl Fn(&T) -> String,
) -> Result<HashMap<String, T>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let mut reader = BufReader::new(File::open(path)?);
    let mut records = HashMap::new();
    let mut committed = 0_u64;
    let mut line_number = 0_usize;
    let mut line = Vec::new();
    while !reader.fill_buf()?.is_empty() {
        line.clear();
        let length = reader
            .by_ref()
            .take(MAX_RECORD_BYTES.saturating_add(1))
            .read_until(b'\n', &mut line)?;
        if u64::try_from(length)? > MAX_RECORD_BYTES {
            anyhow::bail!("JSONL record exceeds 16 MiB: {}", path.display());
        }
        line_number = line_number.saturating_add(1);
        if line.last() != Some(&b'\n') {
            recover_torn_suffix(path, committed)?;
            break;
        }
        committed = committed
            .checked_add(u64::try_from(length)?)
            .context("JSONL offset overflow")?;
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let record: T = serde_json::from_slice(&line)
            .with_context(|| format!("invalid JSON at line {line_number} in {}", path.display()))?;
        records.insert(key(&record), record);
    }
    Ok(records)
}

pub fn append<T: Serialize>(path: &Path, record: &T) -> Result<()> {
    let bytes = serialize_record(record)?;

    let (mut file, created) = open_for_append(path)?;
    file.write_all(&bytes)
        .with_context(|| format!("writing {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("committing {}", path.display()))?;
    if created {
        sync_parent_directory(path)?;
    }
    Ok(())
}

fn open_for_append(path: &Path) -> Result<(File, bool)> {
    match OpenOptions::new().create_new(true).append(true).open(path) {
        Ok(file) => Ok((file, true)),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let file = OpenOptions::new()
                .append(true)
                .open(path)
                .with_context(|| format!("opening {}", path.display()))?;
            Ok((file, false))
        }
        Err(error) => Err(error).with_context(|| format!("creating {}", path.display())),
    }
}

fn recover_torn_suffix(path: &Path, committed: u64) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("opening {} for torn-suffix recovery", path.display()))?;
    file.set_len(committed)
        .with_context(|| format!("truncating torn suffix in {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("committing torn-suffix recovery in {}", path.display()))
}

fn sync_parent_directory(path: &Path) -> Result<()> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    File::open(parent)
        .with_context(|| format!("opening parent directory {}", parent.display()))?
        .sync_all()
        .with_context(|| format!("committing parent directory {}", parent.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use serde_json::{json, Value};

    #[test]
    fn chunked_record_growth_stays_within_allocation_budget() -> Result<()> {
        let mut writer = BoundedBuffer::new(64);
        writer.write_all(b"a")?;
        writer.write_all(&[b'b'; 61])?;
        writer.write_all(b"cc")?;
        assert!(writer.write_all(b"x").is_err());
        let bytes = writer.into_bytes();
        assert_eq!(bytes.len(), 64);
        assert!(
            bytes.capacity() <= 64,
            "record allocation exceeded its budget"
        );
        Ok(())
    }

    #[test]
    fn interrupted_append_recovers_without_losing_committed_records() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("records.jsonl");
        append(&path, &json!({"id":"first"}))?;
        OpenOptions::new()
            .append(true)
            .open(&path)?
            .write_all(b"{\"id\":\"torn\"}")?;
        let records = load::<Value>(&path, |v| v["id"].to_string())?;
        assert_eq!(records.len(), 1);
        assert_eq!(std::fs::read(&path)?, b"{\"id\":\"first\"}\n".to_vec());
        append(&path, &json!({"id":"second"}))?;
        let records = load::<Value>(&path, |v| v["id"].to_string())?;
        assert_eq!(records.len(), 2);
        assert!(records.contains_key("\"first\""));
        assert!(records.contains_key("\"second\""));
        Ok(())
    }

    #[test]
    fn latest_unique_entry_wins() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("records.jsonl");
        append(&path, &json!({"id":"same","value":"old"}))?;
        append(&path, &json!({"id":"same","value":"new"}))?;
        let records = load::<Value>(&path, |v| v["id"].to_string())?;
        assert_eq!(records.len(), 1);
        assert_eq!(
            records
                .get("\"same\"")
                .and_then(|value| value.get("value"))
                .and_then(Value::as_str),
            Some("new")
        );
        Ok(())
    }

    #[test]
    fn committed_interior_corruption_fails_without_recovery() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("records.jsonl");
        let original = b"{\"id\":\"first\"}\n{broken}\n{\"id\":\"last\"}\n";
        std::fs::write(&path, original)?;
        let error = match load::<Value>(&path, |v| v["id"].to_string()) {
            Ok(_) => anyhow::bail!("committed corruption unexpectedly loaded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("line 2"));
        assert_eq!(std::fs::read(&path)?, original);
        Ok(())
    }

    #[test]
    fn oversized_record_fails_without_truncating_the_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("records.jsonl");
        let length = usize::try_from(MAX_RECORD_BYTES)?
            .checked_add(1)
            .context("oversized JSONL test length overflow")?;
        let bytes = vec![b'x'; length];
        std::fs::write(&path, &bytes)?;
        let error = match load::<Value>(&path, Value::to_string) {
            Ok(_) => anyhow::bail!("oversized record unexpectedly loaded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("exceeds 16 MiB"));
        assert_eq!(std::fs::read(&path)?, bytes);
        Ok(())
    }
}
