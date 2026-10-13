use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

use super::fixture::{equal, TestResult};

pub(super) struct Invocation {
    output: Output,
    operation: &'static str,
}

pub(super) fn invoke(
    directory: &Path,
    root: Option<&Path>,
    args: &[&str],
) -> TestResult<Invocation> {
    let operation = match args {
        ["profile", "register", ..] => "register",
        ["profile", "show", ..] => "show",
        ["profile", "select", ..] => "select",
        _ => return Err("profile harness requires an explicit profile operation".into()),
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_census-service"));
    command.current_dir(directory);
    if let Some(root) = root {
        command.arg("--store").arg(root);
    }
    Ok(Invocation {
        output: command.args(args).output()?,
        operation,
    })
}

pub(super) fn success(invocation: Invocation, status: &str) -> TestResult<Value> {
    let Invocation { output, operation } = invocation;
    if !output.status.success() {
        return Err(format!(
            "CLI failed: status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let reply = operational_reply(&output, operation)?;
    equal(reply.get("status"), Some(&Value::String(status.into())))?;
    Ok(reply)
}

pub(super) fn refusal(invocation: Invocation, code: &str) -> TestResult<Value> {
    let Invocation { output, operation } = invocation;
    if output.status.success() {
        return Err(format!(
            "CLI falsely succeeded: {}",
            String::from_utf8_lossy(&output.stdout)
        )
        .into());
    }
    let reply = operational_reply(&output, operation)?;
    equal(reply.get("status"), Some(&serde_json::json!("failed")))?;
    equal(reply.get("code"), Some(&Value::String(code.into())))?;
    match reply.get("message").and_then(Value::as_str) {
        Some(message) if !message.trim().is_empty() => (),
        _ => return Err("operational refusal lost its diagnostic message".into()),
    }
    if output.stderr.is_empty() {
        return Err("operational refusal lost its stderr diagnostic".into());
    }
    Ok(reply)
}

fn operational_reply(output: &Output, operation: &str) -> TestResult<Value> {
    let encoded = output
        .stdout
        .strip_suffix(b"\n")
        .ok_or("operational output lost its newline")?;
    if encoded.first() != Some(&b'{') || encoded.last() != Some(&b'}') {
        return Err("operational output must contain one terminated JSON object".into());
    }
    let reply: Value = serde_json::from_slice(encoded)?;
    equal(reply.get("version"), Some(&serde_json::json!(1)))?;
    equal(
        reply.get("operation"),
        Some(&Value::String(operation.into())),
    )?;
    Ok(reply)
}

pub(super) fn path_text(path: &Path) -> TestResult<&str> {
    path.to_str().ok_or_else(|| "non-UTF8 fixture path".into())
}

pub(super) fn manifest(store: &Store) -> TestResult<RunManifest> {
    Ok(RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("fixture year")?, 1)
            .ok_or("fixture revision")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Witness {
    pub(super) sequence: u64,
    evidence: u64,
    derived: u64,
    tables: String,
    run: Option<RunManifest>,
    pub(super) lineage: Result<String, &'static str>,
}

impl Witness {
    pub(super) fn capture(store: &Store) -> TestResult<Self> {
        let snapshot = store.snapshot();
        let lineage = match snapshot.store_identity() {
            Ok(identity) => Ok(identity.as_str().to_string()),
            Err(census_store::StoreError::StoreLineage(
                census_store::StoreLineageError::Missing,
            )) => Err("missing"),
            Err(error) => return Err(error.into()),
        };
        Ok(Self {
            sequence: snapshot.sequence(),
            evidence: snapshot.evidence_generation(),
            derived: snapshot.derived_generation(),
            tables: snapshot.tables_digest(&Table::ALL)?,
            run: snapshot.run_manifest()?,
            lineage,
        })
    }

    pub(super) fn same_evidence(&self, other: &Self) -> TestResult {
        equal(self.evidence, other.evidence)?;
        equal(self.derived, other.derived)?;
        equal(&self.tables, &other.tables)?;
        equal(&self.run, &other.run)?;
        equal(&self.lineage, &other.lineage)
    }
}

pub(super) fn witness(root: &Path) -> TestResult<Witness> {
    let store = Store::open_with(root, 8_388_608)?;
    Witness::capture(&store)
}

pub(super) fn metadata(root: &Path, id: &str) -> TestResult<Vec<u8>> {
    use fjall::Readable;
    let database = fjall::Database::builder(root.join("fjall")).open()?;
    let meta = database.keyspace("meta", fjall::KeyspaceCreateOptions::default)?;
    let key = format!("profiles:v1:{id}");
    Ok(database
        .snapshot()
        .get(&meta, key.as_bytes())?
        .ok_or("missing fixture metadata")?
        .to_vec())
}

pub(super) fn inject(root: &Path, id: &str, bytes: &[u8]) -> TestResult {
    let database = fjall::Database::builder(root.join("fjall")).open()?;
    let meta = database.keyspace("meta", fjall::KeyspaceCreateOptions::default)?;
    let mut batch = database.batch();
    batch.insert(&meta, format!("profiles:v1:{id}").as_bytes(), bytes);
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()?;
    Ok(())
}
