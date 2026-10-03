use anyhow::{ensure, Context, Result};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub const LIMIT: u64 = 32 * 1024 * 1024;

#[cfg(test)]
mod tests;

pub fn read(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).with_context(|| format!("reading {}", path.display()))?;
    ensure!(
        file.metadata()?.len() <= LIMIT,
        "artifact exceeds 32 MiB: {}",
        path.display()
    );
    let mut bytes = Vec::new();
    file.take(LIMIT.saturating_add(1)).read_to_end(&mut bytes)?;
    ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "artifact grew beyond budget"
    );
    Ok(bytes)
}

pub fn json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&read(path)?)?)
}

pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "RESOURCE_LIMIT: artifact publication exceeds 32 MiB: {}",
        path.display()
    );
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    File::open(path.parent().context("artifact parent absent")?)?.sync_all()?;
    Ok(())
}

pub fn publish(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("pending");
    write(&temporary, &encode(value, true)?)?;
    std::fs::rename(temporary, path)?;
    File::open(path.parent().context("artifact parent absent")?)?.sync_all()?;
    Ok(())
}

pub fn append(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut bytes = encode(value, false)?;
    ensure!(
        u64::try_from(bytes.len())? < LIMIT,
        "RESOURCE_LIMIT: artifact record plus newline exceeds 32 MiB"
    );
    bytes.try_reserve(1).context("reserving artifact newline")?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    lock_writer(&file)?;
    let final_length = file
        .metadata()?
        .len()
        .checked_add(u64::try_from(bytes.len())?)
        .context("artifact append length overflow")?;
    ensure!(
        final_length <= LIMIT,
        "RESOURCE_LIMIT: artifact append exceeds 32 MiB: {}",
        path.display()
    );
    file.write_all(&bytes)?;
    file.sync_data()?;
    file.unlock()?;
    Ok(())
}

fn lock_writer(file: &File) -> Result<()> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(1))
        .context("artifact writer deadline overflow")?;
    for _ in 0..100 {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(anyhow::anyhow!("artifact writer lock unavailable: {error}")),
        }
    }
    Err(anyhow::anyhow!(
        "artifact writer lock exceeded one-second contention budget"
    ))
}

pub fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn now() -> String {
    chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now()).to_rfc3339()
}

struct Encoded(Vec<u8>);

impl Write for Encoded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let size = self
            .0
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("artifact encoded length overflow"))?;
        if u64::try_from(size).map_err(std::io::Error::other)? > LIMIT {
            return Err(std::io::Error::other(
                "RESOURCE_LIMIT: encoded artifact exceeds 32 MiB",
            ));
        }
        self.0
            .try_reserve(bytes.len())
            .map_err(std::io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encode(value: &impl Serialize, pretty: bool) -> Result<Vec<u8>> {
    let mut encoded = Encoded(Vec::new());
    if pretty {
        serde_json::to_writer_pretty(&mut encoded, value)?;
    } else {
        serde_json::to_writer(&mut encoded, value)?;
    }
    ensure!(
        u64::try_from(encoded.0.len())? <= LIMIT,
        "RESOURCE_LIMIT: final encoded artifact exceeds 32 MiB"
    );
    Ok(encoded.0)
}
