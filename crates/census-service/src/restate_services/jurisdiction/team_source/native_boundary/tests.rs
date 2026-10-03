use std::error::Error;
use std::fs::{File, OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::error::BoundaryError;
use super::{ledger, marker, read_armed, Armed, Marker};

type TestResult = Result<(), Box<dyn Error>>;
const OPERATION: &str = "jurisdiction:RI:2026-27:1/teams/milesplit";

mod cancellation;
mod config;
mod control;
mod markers;

struct Fixture {
    root: tempfile::TempDir,
    config: PathBuf,
}

impl Fixture {
    fn new(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let root = tempfile::tempdir()?;
        std::fs::set_permissions(root.path(), Permissions::from_mode(0o700))?;
        let config = root
            .path()
            .canonicalize()?
            .join("native-source-boundary-config.json");
        write_new(&config, bytes)?;
        Ok(Self { root, config })
    }

    fn armed(&self) -> Result<Armed, BoundaryError> {
        read_armed(&self.config)
    }

    fn marker(&self) -> PathBuf {
        self.root.path().join(marker::BASENAME)
    }

    fn pending(&self) -> PathBuf {
        self.root.path().join(marker::PENDING)
    }
}

fn configuration(attempt: u8, timeout: u64) -> Value {
    json!({"schema": 1, "operation": OPERATION, "attempt": attempt, "timeout_seconds": timeout})
}

fn fixture(attempt: u8, timeout: u64) -> Result<Fixture, Box<dyn Error>> {
    Fixture::new(&serde_json::to_vec(&configuration(attempt, timeout))?)
}

fn identity() -> ledger::Identity {
    ledger::Identity {
        request_digest: "0123456789abcdef".repeat(4),
        observed_on: "2026-10-02".to_string(),
    }
}

fn reserved_marker(attempt: u8) -> Result<Marker, BoundaryError> {
    Marker::new(OPERATION, attempt, &identity())
}

fn expected_marker(attempt: u8) -> Value {
    json!({
        "schema": 1,
        "phase": "teams_reserved_before_acquisition",
        "operation": OPERATION,
        "attempt": attempt,
        "request_digest": identity().request_digest,
        "observed_on": "2026-10-02"
    })
}

fn write_new(path: &Path, bytes: &[u8]) -> TestResult {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    File::open(path.parent().ok_or("missing fixture parent")?)?.sync_all()?;
    Ok(())
}

fn read_marker(fixture: &Fixture) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(fixture.marker())?)?)
}

fn terminal(error: restate_sdk::prelude::TerminalError) -> std::io::Error {
    std::io::Error::other(error.to_string())
}
