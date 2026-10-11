use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub(super) fn compute_file_hash(path: &Path) -> Result<(u64, String)> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::new(&file);

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut total = 0u64;

    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("reading {}", path.display()))?;
        if n == 0 {
            break;
        }
        let chunk =
            u64::try_from(n).map_err(|_| anyhow::anyhow!("hashed chunk does not fit in u64"))?;
        total = total.saturating_add(chunk);
        let bytes = buf.get(..n).context("hashed chunk exceeds buffer length")?;
        hasher.update(bytes);
    }

    let result = hasher.finalize();
    let hex = hex_encode(&result);

    Ok((total, hex))
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::new();
    for &b in bytes {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}
