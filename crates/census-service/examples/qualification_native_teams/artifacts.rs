use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod encoding;
#[cfg(test)]
mod tests;

pub const MODEL: &str = "openai-codex/gpt-6.1-sol";
pub const MAX_ARTIFACT: u64 = 32 * 1024 * 1024;

pub fn now() -> Result<String> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(format!(
        "{}.{:09}Z-unix",
        elapsed.as_secs(),
        elapsed.subsec_nanos()
    ))
}

pub fn write_new(path: &Path, body: &[u8]) -> Result<()> {
    checked_size(0, body.len())?;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(body)?;
    file.sync_all()?;
    Ok(())
}

pub fn write_json(path: &Path, value: &Value) -> Result<()> {
    write_new(path, &encoding::pretty(value)?)
}

pub fn read_bounded(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.len() <= MAX_ARTIFACT,
        "artifact exceeds read budget: {}",
        path.display()
    );
    let mut text = String::new();
    file.take(MAX_ARTIFACT).read_to_string(&mut text)?;
    Ok(text)
}

pub fn append(path: &Path, value: &Value) -> Result<()> {
    let encoded = encoding::record(value)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    lock_writer(&file)?;
    checked_size(file.metadata()?.len(), encoded.len())?;
    file.write_all(&encoded)?;
    file.sync_data()?;
    file.unlock()?;
    Ok(())
}

fn lock_writer(file: &std::fs::File) -> Result<()> {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(1))
        .context("artifact writer lock deadline overflow")?;
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

fn checked_size(existing: u64, encoded: usize) -> Result<u64> {
    let length = existing
        .checked_add(u64::try_from(encoded)?)
        .context("artifact resource-limit: final size overflow")?;
    ensure!(
        length <= MAX_ARTIFACT,
        "artifact resource-limit: final encoded size exceeds 32 MiB"
    );
    Ok(length)
}

pub fn rows(value: &Value) -> Result<&[Value]> {
    value
        .get("rows")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .or_else(|| value.as_array().map(Vec::as_slice))
        .context("native admin query has neither rows array nor array response")
}

pub fn oracle(name: &str, passed: bool, measured: Value) -> Value {
    json!({"name": name, "status": if passed { "PASS" } else { "UNPROVEN" }, "measured": measured})
}
