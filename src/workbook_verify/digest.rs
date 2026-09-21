use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, BufReader, Write},
    path::Path,
};

pub(super) fn hash_file(path: &Path) -> Result<String> {
    let mut input = BufReader::new(File::open(path).context("opening workbook for hashing")?);
    let mut sink = HashWriter {
        digest: Sha256::new(),
    };
    io::copy(&mut input, &mut sink).context("hashing workbook bytes")?;
    Ok(format!("{:x}", sink.digest.finalize()))
}

struct HashWriter {
    digest: Sha256,
}

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.digest.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
