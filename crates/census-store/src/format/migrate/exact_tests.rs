use std::collections::BTreeMap;
use std::path::Path;

use census_domain::model::{CanonicalPerformance, CoverageRow, CoverageScope, Mark};
use fjall::Keyspace;
use serde_json::{json, Value};

use crate::keys::{derived_key, observation_key};
use crate::{generation, meta, Store, StoreError, Table};

use super::tests::{erase_current_schema, raw_import, TestResult};

mod owned;
mod owned_duplicates;
mod preservation;
mod resume;

fn performance(id: &str, time: Value) -> Value {
    json!({"id":id,"athlete":"ath_subject_historic","team":"team_historic",
        "event":"evt_historic","meet":"meet_historic","date":"2026-05-01",
        "mark":{"TimeSeconds":time},"evidence":[],"source_key":"historic:1",
        "raw_mark":"10.941"})
}

fn version(store: &Store, version: &str) -> TestResult {
    let mut batch = store.db.batch();
    meta::put_text(&mut batch, &store.meta, "schema:store_version", version);
    batch.commit()?;
    Ok(())
}

fn seed(root: &Path, rows: &[Value]) -> TestResult {
    let store = Store::open(root)?;
    store.append_many(Table::Performances, rows)?;
    version(&store, "1")?;
    Ok(())
}

fn raw_performance(root: &Path, id: &str, sequence: u64) -> TestResult<Vec<u8>> {
    let (_, entities, _, _, _) = crate::open_keyspaces(root, crate::format::DEFAULT_CACHE_BYTES)?;
    Ok(entities
        .get(observation_key(Table::Performances, id, sequence))?
        .ok_or("missing performance")?
        .to_vec())
}

fn all_bytes(keyspace: &Keyspace) -> TestResult<BTreeMap<Vec<u8>, Vec<u8>>> {
    keyspace
        .iter()
        .map(|guard| -> TestResult<_> {
            let (key, value) = guard.into_inner()?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect()
}

fn expected_time() -> Value {
    json!({"nanoseconds":10_940_000_000_i64,"precision":2})
}

#[test]
fn schema1_integer_centiseconds_become_exact_nanoseconds_without_claiming_raw_precision(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let original = performance("perf_historic", json!(1094));
    seed(dir.path(), std::slice::from_ref(&original))?;
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::MigrationRequired { .. })
    ));
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.from_schema, report.to_schema, report.rewritten_rows, report.dropped_rows), (Some(1), 2, 1, 0));
    let migrated: Value =
        serde_json::from_slice(&raw_performance(dir.path(), "perf_historic", 0)?)?;
    let mut expected = original;
    expected["mark"]["TimeSeconds"] = expected_time();
    check!(eq; migrated, expected);
    let store = Store::open(dir.path())?;
    let rows = store.scan::<CanonicalPerformance>(Table::Performances)?;
    let Mark::TimeSeconds(time) = &rows.first().ok_or("missing migrated performance")?.mark else {
        return Err("migrated mark lost canonical time".into());
    };
    check!(eq; (time.value(), time.precision()), (10_940_000_000, 2));
    Ok(())
}

#[test]
fn an_already_new_shape_is_byte_identical_and_schema2_migration_is_a_noop() -> TestResult {
    let dir = tempfile::tempdir()?;
    seed(
        dir.path(),
        &[performance(
            "perf_historic",
            json!({"nanoseconds":10_941_000_000_i64,"precision":3}),
        )],
    )?;
    let original = raw_performance(dir.path(), "perf_historic", 0)?;
    let report = Store::migrate(dir.path())?;
    check!(eq; report.rewritten_rows, 0);
    check!(eq; raw_performance(dir.path(), "perf_historic", 0)?, original);
    let store = Store::open(dir.path())?;
    let before_meta = all_bytes(&store.meta)?;
    let before_journal = all_bytes(&store.journal)?;
    drop(store);
    let current = Store::migrate(dir.path())?;
    check!(eq; (current.already_current, current.rewritten_rows, current.dropped_rows), (true, 0, 0));
    let store = Store::open(dir.path())?;
    check!(eq; all_bytes(&store.meta)?, before_meta);
    check!(eq; all_bytes(&store.journal)?, before_journal);
    Ok(())
}

#[test]
fn malformed_or_overflowing_times_keep_schema1_unfinished_and_refuse_open() -> TestResult {
    let cases = [
        json!(0),
        json!(-1),
        json!(922_337_203_686_i64),
        json!(i64::MAX),
        json!(u64::MAX),
        json!(1094.0),
        json!("1094"),
        Value::Null,
        json!({"nanoseconds":0,"precision":2}),
        json!({"nanoseconds":1,"precision":2}),
        json!({"nanoseconds":10_940_000_000_i64,"precision":10}),
    ];
    cases.into_iter().try_for_each(|time| -> TestResult {
        let dir = tempfile::tempdir()?;
        seed(dir.path(), &[performance("perf_historic", time)])?;
        let original = raw_performance(dir.path(), "perf_historic", 0)?;
        check!(matches!(
            Store::migrate(dir.path()),
            Err(StoreError::Refused { .. } | StoreError::Json { .. })
        ));
        let format = Store::format(dir.path())?;
        check!(eq; (format.schema_version, format.migration_target), (Some(1), Some(2)));
        check!(eq; raw_performance(dir.path(), "perf_historic", 0)?, original);
        check!(matches!(
            Store::open(dir.path()),
            Err(StoreError::MigrationRequired { .. })
        ));
        Ok(())
    })
}
