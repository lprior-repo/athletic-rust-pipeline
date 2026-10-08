use anyhow::{ensure, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::ledger::{Measurements, Request};
use super::scenario::Facts;
use super::{CLOCK_TOLERANCE_MS, DELAY_MS, INFLIGHT_BUDGET};

pub(super) fn directory() -> Result<PathBuf> {
    let temporary = match std::env::var_os("CENSUS_ORIGIN_BUDGET_EVIDENCE") {
        Some(root) => tempfile::Builder::new()
            .prefix("native-origin-budget-")
            .tempdir_in(root)?,
        None => tempfile::Builder::new()
            .prefix("native-origin-budget-")
            .tempdir()?,
    };
    let directory = temporary.keep();
    println!("EVIDENCE: {}", directory.display());
    Ok(directory)
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Identity {
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Identities {
    pub cli: Identity,
    pub regression: Identity,
}

pub(super) fn identities(binary: &Path) -> Result<Identities> {
    Ok(Identities {
        cli: identity(binary)?,
        regression: identity(&std::env::current_exe()?)?,
    })
}

pub(super) fn identity(path: &Path) -> Result<Identity> {
    let mut file = File::open(path)?;
    let bytes = file.metadata()?.len();
    ensure!(
        bytes > 0 && bytes <= 536_870_912,
        "executable exceeds 512MiB qualification bound"
    );
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    let mut read = 0_u64;
    for _ in 0..8193 {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            ensure!(read == bytes, "executable length changed while hashing");
            return Ok(Identity {
                path: std::fs::canonicalize(path)?,
                sha256: format!("{:x}", hash.finalize()),
                bytes,
            });
        }
        read = read
            .checked_add(u64::try_from(count)?)
            .ok_or_else(|| anyhow::anyhow!("executable size overflow"))?;
        ensure!(read <= bytes, "executable grew while hashing");
        hash.update(
            buffer
                .get(..count)
                .ok_or_else(|| anyhow::anyhow!("executable read size"))?,
        );
    }
    Err(anyhow::anyhow!("executable hash read bound exceeded"))
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn publish(
    directory: &Path,
    origin: &str,
    identities: Identities,
    facts: Facts,
    requests: Vec<Request>,
    measurements: Measurements,
) -> Result<()> {
    let certificate = serde_json::json!({
        "schema":1,
        "scenario":8,
        "scope":"independent native CLI acquisition workflows on one host sharing the production working-directory origin lock; no serving Restate endpoints or distributed coordination exercised",
        "origin":origin,
        "configuration":{"delay_ms":DELAY_MS,"clock_tolerance_ms":CLOCK_TOLERANCE_MS,
            "inflight_budget":INFLIGHT_BUDGET,"burst_budget":1,"rival_workflows":2,
            "origin_lock_root":"var/locks","server_connection_bound":8,"request_deadline_seconds":90},
        "identities":identities,
        "facts":facts,
        "requests":requests,
        "measurements":measurements,
        "http_ledger_sha256":digest(&std::fs::read(directory.join("http-ledger.jsonl"))?),
    });
    let path = directory.join("origin-budget.json");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    file.write_all(&serde_json::to_vec_pretty(&certificate)?)?;
    file.sync_all()?;
    File::open(directory)?.sync_all()?;
    println!("CERTIFICATE: {}", path.display());
    Ok(())
}
