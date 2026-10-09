use anyhow::{ensure, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub(super) fn write(directory: &Path, name: &str, value: &serde_json::Value) -> Result<()> {
    let path = directory.join(name);
    let encoded = serde_json::to_vec_pretty(value)?;
    ensure!(
        encoded.len() <= 1_048_576,
        "certificate {name} exceeded 1MiB"
    );
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    File::open(directory)?.sync_all()?;
    println!("EVIDENCE: {}", path.display());
    Ok(())
}
