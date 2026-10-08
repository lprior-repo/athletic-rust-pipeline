use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Duration;

use census_store::{Receipt, Store, Table};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::super::process::census_command;
use super::super::{TestResult, ATHLETICLIVE_FIXTURE};

pub(super) struct SourceInput {
    pub input: PathBuf,
    pub metadata: PathBuf,
}

impl SourceInput {
    pub fn write(root: &Path, count: usize) -> TestResult<Self> {
        let input = root.join("athleticlive-meets.csv");
        let metadata = root.join("synthetic-capture.meta.json");
        let header = ATHLETICLIVE_FIXTURE
            .lines()
            .next()
            .ok_or("missing captured CSV header")?;
        let mut body = format!("{header}\n");
        for ordinal in 0..count {
            let id = 50_001_usize
                .checked_add(ordinal)
                .ok_or("synthetic source id overflow")?;
            body.push_str(&format!("athleticlive,{id},,Synthetic Recovery Meet {ordinal},,Illinois,2025-09-15T04:00:00Z,,True\n"));
        }
        std::fs::write(&input, &body)?;
        let producer = json!({
            "url":"https://synthetic.athleticlive.example/meets.csv","method":"GET","status":200,
            "bytes":body.len(),"content_digest":format!("{:x}",Sha256::digest(body.as_bytes())),
            "fetched_at":"2026-09-20T00:00:00Z","content_type":"text/csv"
        });
        std::fs::write(&metadata, serde_json::to_vec(&producer)?)?;
        Ok(Self { input, metadata })
    }

    pub fn args(&self) -> TestResult<[&str; 6]> {
        Ok([
            "provider",
            "athleticlive",
            "--input",
            self.input.to_str().ok_or("input path is not UTF-8")?,
            "--input-metadata",
            self.metadata.to_str().ok_or("metadata path is not UTF-8")?,
        ])
    }
}

pub(super) fn receipts(store: &Store) -> TestResult<BTreeMap<String, Receipt>> {
    let mut held = BTreeMap::new();
    store.for_each_receipt(100_000, |receipt| {
        held.insert(receipt.operation.clone(), receipt);
        Ok(())
    })?;
    Ok(held)
}

pub(super) fn kill_at_boundary(root: &Path, args: &[&str], phase: &str) -> TestResult<Value> {
    let marker = root.with_extension("boundary.json");
    let log = root.with_extension("worker.stderr.log");
    let mut child = census_command(root)
        .args(args)
        .env("CENSUS_NATIVE_WORKER_BOUNDARY", phase)
        .env("CENSUS_NATIVE_WORKER_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&log)?)
        .spawn()?;
    let pid = child.id();
    let reached = wait_for_marker(&mut child, &marker);
    let delivery = child.kill();
    let reaped = child.wait();
    let (marker, status) = match (reached, delivery, reaped) {
        (Ok(marker), Ok(()), Ok(status)) => (marker, status),
        (reached, delivery, reaped) => {
            return Err(format!("native boundary or owned child cleanup failed: reached={reached:?}; delivery={delivery:?}; reaped={reaped:?}").into());
        }
    };
    check!(eq; status.signal(), Some(9), "owned interrupted worker must really receive SIGKILL");
    check!(eq; marker.get("schema").and_then(Value::as_u64), Some(1));
    check!(eq; marker.get("pid").and_then(Value::as_u64), Some(u64::from(pid)));
    check!(eq; marker.get("phase").and_then(Value::as_str), Some(phase));
    check!(marker
        .get("operation")
        .and_then(Value::as_str)
        .is_some_and(|id| !id.is_empty()));
    check!(marker
        .get("input_digest")
        .and_then(Value::as_str)
        .is_some_and(|digest| digest.len() == 64));
    Ok(marker)
}

fn wait_for_marker(child: &mut Child, path: &Path) -> TestResult<Value> {
    for _ in 0..3_000 {
        if path.is_file() {
            let bytes = std::fs::read(path)?;
            check!(
                bytes.len() <= 65_536,
                "native worker marker exceeded its protocol bound"
            );
            check!(
                child.try_wait()?.is_none(),
                "worker exited before reached boundary fault delivery"
            );
            return Ok(serde_json::from_slice(&bytes)?);
        }
        if let Some(status) = child.try_wait()? {
            return Err(format!("worker exited before reaching boundary: {status}").into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("worker did not reach the requested actual transaction boundary within30seconds".into())
}

pub(super) fn certificate(phase: &str, value: Value) -> TestResult {
    let Some(directory) = std::env::var_os("CENSUS_NATIVE_RECOVERY_EVIDENCE") else {
        return Ok(());
    };
    let path = PathBuf::from(directory).join(format!("{phase}.json"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    file.write_all(&serde_json::to_vec_pretty(&value)?)?;
    file.sync_all()?;
    Ok(())
}

pub(super) const DERIVED: [Table; 4] = [
    Table::SourceIdentities,
    Table::Conflicts,
    Table::ReviewCases,
    Table::Coverage,
];
