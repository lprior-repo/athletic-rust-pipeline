use std::error::Error;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use census_store::{Store, StoreError, Table};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::super::{consolidate_armed, consolidate_tables, ORDINARY_ERROR_ENVIRONMENT};
use super::{
    operation, reached, BoundaryError, BOUNDARY_ENVIRONMENT, HOLD_ENVIRONMENT, MARKER_ENVIRONMENT,
    OPERATION_PREFIX, PHASE, SCHEMA,
};

mod publication;

type TestResult = Result<(), Box<dyn Error>>;
type SnapshotBytes = Vec<(String, Vec<u8>)>;

const KEY: &str = "national:2026-27:1:WI";
const UNRELATED_KEY: &str = "national:2026-27:1:CA";
const CONSOLE_ENTRY: &str = include_str!("../../../../../../docs/NATIONAL-CENSUS-FAULTS.md");

static ENVIRONMENT_LOCK: Mutex<()> = Mutex::new(());

struct Arming {
    _guard: MutexGuard<'static, ()>,
}

impl Arming {
    fn clear() -> Self {
        let guard = match ENVIRONMENT_LOCK.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };
        Self::unset();
        Self { _guard: guard }
    }

    fn boundary(&self, key: &str, marker: &Path, hold_millis: u64) {
        std::env::set_var(BOUNDARY_ENVIRONMENT, operation(key));
        std::env::set_var(MARKER_ENVIRONMENT, marker);
        std::env::set_var(HOLD_ENVIRONMENT, hold_millis.to_string());
    }

    fn ordinary_error(&self) {
        std::env::set_var(ORDINARY_ERROR_ENVIRONMENT, "1");
    }

    fn clear_variables(&self) {
        Self::unset();
    }

    fn unset() {
        for name in [
            BOUNDARY_ENVIRONMENT,
            MARKER_ENVIRONMENT,
            HOLD_ENVIRONMENT,
            ORDINARY_ERROR_ENVIRONMENT,
        ] {
            std::env::remove_var(name);
        }
    }
}

impl Drop for Arming {
    fn drop(&mut self) {
        Self::unset();
    }
}

#[test]
fn consolidate_boundary_unarmed_consolidation_is_byte_identical_to_normal_behavior() -> TestResult {
    let _arming = Arming::clear();
    let directory = tempfile::tempdir()?;
    let marker = directory.path().join("consolidate-boundary.json");
    let store = Store::open(directory.path())?;
    let requested = Table::ALL.to_vec();
    let control = requested.iter().try_fold(
        Vec::new(),
        |mut control, table| -> Result<Vec<(String, usize)>, StoreError> {
            let path = store.out_dir().join(format!("{}.jsonl", table.file()));
            let rows = store.consolidate_table(*table, &path)?.rows;
            control.push((table.file().to_string(), rows));
            Ok(control)
        },
    )?;
    let before = snapshot_bytes(&store)?;
    let consolidated = consolidate_armed(&store, &requested, &operation(KEY))?;
    let observed: Vec<(String, usize)> = consolidated
        .iter()
        .map(|table| (table.table.clone(), table.rows))
        .collect();
    check!(
        eq;
        observed,
        control,
        "the unarmed consolidation differs from the direct table-wise control"
    );
    let after = snapshot_bytes(&store)?;
    check!(eq; after, before, "the unarmed boundary rewrote snapshot bytes");
    check!(
        !marker.exists(),
        "the unarmed boundary published a reached-boundary marker"
    );
    Ok(())
}

#[test]
fn consolidate_boundary_armed_operation_reaches_once_while_a_mismatched_or_expired_scope_performs_no_fault(
) -> TestResult {
    let arming = Arming::clear();
    let directory = tempfile::tempdir()?;
    let marker = directory.path().join("consolidate-boundary.json");
    let store = Store::open(directory.path())?;
    let requested = vec![Table::Schools, Table::Meets];
    arming.boundary(KEY, &marker, 1);
    let unrelated = consolidate_armed(&store, &requested, &operation(UNRELATED_KEY))?;
    check!(
        eq;
        unrelated.len(),
        requested.len(),
        "a mismatched arming scope skipped the consolidation"
    );
    check!(
        !marker.exists(),
        "a mismatched arming scope published a reached-boundary marker"
    );
    let consolidated = consolidate_tables(&store, &requested)?;
    match reached(&store, &operation(KEY), &consolidated) {
        Err(BoundaryError::HoldExpired {
            operation: held,
            millis,
        }) => {
            check!(
                held == operation(KEY),
                "the expired hold must name the armed operation: {held}"
            );
            check!(eq; millis, 1, "the expired hold must honor its bound");
        }
        Err(other) => {
            return Err(format!(
                "the armed operation answered {other} instead of a reached-boundary hold"
            )
            .into())
        }
        Ok(()) => {
            return Err("the armed operation returned success without reaching its boundary".into())
        }
    }
    check!(
        marker.is_file(),
        "the armed operation published no reached-boundary marker"
    );
    let value = marker_value(&marker)?;
    check!(
        eq;
        value.get("schema").and_then(Value::as_u64),
        Some(u64::from(SCHEMA)),
        "the marker schema must name the contract version"
    );
    check!(
        eq;
        value.get("phase").and_then(Value::as_str),
        Some(PHASE),
        "the marker phase must name the reached boundary"
    );
    check!(
        eq;
        value.get("pid").and_then(Value::as_u64),
        Some(u64::from(std::process::id())),
        "the marker pid must be the process that reached the boundary"
    );
    check!(
        eq;
        value.get("operation").and_then(Value::as_str),
        Some(operation(KEY).as_str()),
        "the marker operation must be the armed operation"
    );
    bind_evidence(&value, &store, &requested)?;
    std::fs::remove_file(&marker)?;
    let consumed = consolidate_armed(&store, &requested, &operation(KEY))?;
    check!(
        eq;
        consumed.len(),
        requested.len(),
        "the consumed arming scope faulted the next operation"
    );
    check!(
        !marker.exists(),
        "the consumed arming scope published a second reached-boundary marker"
    );
    let unrelated = consolidate_armed(&store, &requested, &operation(UNRELATED_KEY))?;
    check!(
        eq;
        unrelated.len(),
        requested.len(),
        "an unrelated operation inherited the consumed arming scope"
    );
    check!(
        !marker.exists(),
        "an unrelated operation published a reached-boundary marker"
    );
    arming.clear_variables();
    Ok(())
}

#[test]
fn consolidate_boundary_injected_error_stays_separately_named_and_typed() -> TestResult {
    let arming = Arming::clear();
    let directory = tempfile::tempdir()?;
    let marker = directory.path().join("both.json");
    let store = Store::open(directory.path())?;
    let requested = vec![Table::Schools];
    arming.ordinary_error();
    match consolidate_tables(&store, &requested) {
        Err(StoreError::Invariant { detail }) => {
            check!(
                detail.contains("ordinary"),
                "the ordinary injection must name itself: {detail}"
            );
        }
        Err(other) => {
            return Err(format!(
                "the ordinary injection answered {other} instead of its typed store error"
            )
            .into())
        }
        Ok(_) => return Err("the ordinary injected error did not fire".into()),
    }
    arming.boundary(KEY, &marker, 1);
    match consolidate_tables(&store, &requested) {
        Err(StoreError::Invariant { detail }) => {
            check!(
                detail.contains("ordinary"),
                "the ordinary injection must win while the boundary is armed: {detail}"
            );
        }
        Err(other) => {
            return Err(format!(
                "the armed operation answered {other} instead of the ordinary error"
            )
            .into())
        }
        Ok(_) => return Err("the armed operation skipped the ordinary injected error".into()),
    }
    check!(
        !marker.exists(),
        "the ordinary injected error published a reached-boundary marker"
    );
    arming.clear_variables();
    let normal = consolidate_tables(&store, &requested)?;
    check!(
        eq;
        normal.len(),
        requested.len(),
        "clearing the injection left the operation abnormal"
    );
    Ok(())
}

#[test]
fn consolidate_boundary_console_entry_documents_the_boundary_and_arming_scope() -> TestResult {
    let scope = format!("{OPERATION_PREFIX}{{key}}");
    for required in [
        BOUNDARY_ENVIRONMENT,
        MARKER_ENVIRONMENT,
        HOLD_ENVIRONMENT,
        PHASE,
        OPERATION_PREFIX,
        scope.as_str(),
    ] {
        check!(
            CONSOLE_ENTRY.contains(required),
            "docs/NATIONAL-CENSUS-FAULTS.md does not document {required}"
        );
    }
    for required in ["arming", "replay"] {
        check!(
            CONSOLE_ENTRY.contains(required),
            "docs/NATIONAL-CENSUS-FAULTS.md does not document the {required} expectation"
        );
    }
    Ok(())
}

fn marker_value(path: &Path) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn snapshot_bytes(store: &Store) -> Result<SnapshotBytes, Box<dyn Error>> {
    let mut snapshots = Vec::new();
    for table in Table::ALL {
        let path = store.out_dir().join(format!("{}.jsonl", table.file()));
        snapshots.push((table.file().to_string(), std::fs::read(&path)?));
    }
    Ok(snapshots)
}

fn bind_evidence(value: &Value, store: &Store, requested: &[Table]) -> TestResult {
    let entries = value
        .get("tables")
        .and_then(Value::as_array)
        .ok_or("the marker carries no table inventory")?;
    check!(
        eq;
        entries.len(),
        requested.len(),
        "the marker inventory must match the requested tables"
    );
    for table in requested {
        let name = table.file();
        let entry = entries
            .iter()
            .find(|entry| entry.get("table").and_then(Value::as_str) == Some(name))
            .ok_or_else(|| format!("the marker does not name {name}"))?;
        let digest = entry
            .get("digest")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("the marker carries no digest for {name}"))?;
        let path = store.out_dir().join(format!("{name}.jsonl"));
        let durable = format!("{:x}", Sha256::digest(std::fs::read(&path)?));
        check!(
            eq;
            digest,
            durable.as_str(),
            "the marker digest for {name} must bind the durable snapshot bytes"
        );
    }
    Ok(())
}
