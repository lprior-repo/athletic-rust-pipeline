use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

pub(super) fn hash_file(path: &Path) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut digest = Sha256::new();
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            break;
        }
        digest.update(chunk);
        let length = chunk.len();
        reader.consume(length);
    }
    Ok(format!("{:x}", digest.finalize()))
}
