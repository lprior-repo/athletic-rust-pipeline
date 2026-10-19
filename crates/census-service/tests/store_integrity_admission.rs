#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const HEADER: &[u8] = b"manual-context-admission-v1\0header";

fn cli(root: &Path, arguments: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(root)
        .args(arguments)
        .output()
}

fn current(root: &Path) -> TestResult {
    for arguments in [
        ["capture-protocol", "init"],
        ["capture-protocol", "enable-admission"],
    ] {
        let output = cli(root, &arguments)?;
        if !output.status.success() {
            return Err(String::from_utf8(output.stderr)?.into());
        }
    }
    Ok(())
}

fn journal(root: &Path) -> TestResult<(Database, Keyspace)> {
    let database = Database::builder(root.join("fjall")).open()?;
    if !database.keyspace_exists("journal") {
        return Err("current source journal is absent".into());
    }
    let journal = database.keyspace("journal", KeyspaceCreateOptions::default)?;
    Ok((database, journal))
}

fn read_row(root: &Path, key: &[u8]) -> TestResult<Vec<u8>> {
    let (_database, journal) = journal(root)?;
    Ok(journal
        .get(key)?
        .ok_or("retained journal row absent")?
        .to_vec())
}

fn write_row(root: &Path, key: &[u8], bytes: &[u8]) -> TestResult {
    let (database, journal) = journal(root)?;
    journal.insert(key, bytes)?;
    database.persist(PersistMode::SyncAll)?;
    Ok(())
}

#[test]
fn invalid_admission_policy_is_refused_without_repairing_the_owned_header() -> TestResult {
    let directory = tempfile::tempdir()?;
    current(directory.path())?;
    let mut header: serde_json::Value =
        serde_json::from_slice(&read_row(directory.path(), HEADER)?)?;
    *header.get_mut("policy").ok_or("admission policy absent")? = "unsupported-policy".into();
    let changed = serde_json::to_vec(&header)?;
    write_row(directory.path(), HEADER, &changed)?;
    let output = cli(directory.path(), &["store-integrity"])?;
    check!(eq; output.status.code(), Some(1));
    check!(eq; read_row(directory.path(), HEADER)?, changed);
    Ok(())
}

#[test]
fn an_unowned_header_without_an_event_payload_is_still_reported_corrupt() -> TestResult {
    let directory = tempfile::tempdir()?;
    current(directory.path())?;
    let key = b"unknown-integrity-protocol\0header";
    let raw = br#"{"version":1}"#;
    write_row(directory.path(), key, raw)?;
    let output = cli(directory.path(), &["store-integrity"])?;
    check!(eq; output.status.code(), Some(1));
    check!(std::str::from_utf8(&output.stdout)?
        .lines()
        .any(|line| line == "ok\tfalse"));
    check!(eq; read_row(directory.path(), key)?.as_slice(), raw);
    Ok(())
}
