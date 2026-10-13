use crate::{Store, StoreError};
use census_domain::model::RunManifest;

use super::support::{
    equal, fixture_dir, inject, raw_binding, seed, TestResult, Witness, HISTORICAL,
};

fn schema1_fixture(retained: Option<&[u8]>) -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    if let Some(raw) = retained {
        inject(&store, raw)?;
    }
    let expected: Option<RunManifest> = retained.map(serde_json::from_slice).transpose()?;
    let mut batch = store.db.batch();
    batch.insert(&store.meta, "schema:store_version", b"1".as_slice());
    batch.commit()?;
    let mut before = Witness::capture(&store)?;
    drop(store);
    match Store::open_with(root.path(), 8_388_608) {
        Err(StoreError::MigrationRequired { .. }) => {}
        Err(other) => return Err(format!("unexpected schema1 opener refusal: {other:?}").into()),
        Ok(_) => {
            return Err("schema1 fixture opened without its required offline migration".into())
        }
    }
    let report = Store::migrate_with(root.path(), 8_388_608)?;
    equal(
        (report.from_schema, report.to_schema, report.to_key_format),
        (Some(1), 2, 1),
    )?;
    equal(
        (
            report.already_current,
            report.rewritten_rows,
            report.dropped_rows,
            report.integrity_ok,
        ),
        (false, 0, 0, true),
    )?;
    let migrated = Store::open_with(root.path(), 8_388_608)?;
    equal(raw_binding(&migrated)?, retained.map(<[u8]>::to_vec))?;
    equal(migrated.run_manifest()?, expected.clone())?;
    equal(migrated.snapshot().run_manifest()?, expected)?;
    let after = Witness::capture(&migrated)?;
    equal(
        (after.evidence, after.derived),
        (before.evidence, before.derived),
    )?;
    equal(&after.digest, &before.digest)?;
    equal(&after.entities, &before.entities)?;
    equal(&after.journal, &before.journal)?;
    equal(&after.receipts, &before.receipts)?;
    let row = before
        .metadata
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"schema:store_version")
        .ok_or("schema1 fixture has no schema version")?;
    row.1 = b"2".to_vec();
    equal(&after.metadata, &before.metadata)?;
    equal(after.format.migration_target, None)?;
    drop(migrated);
    let again = Store::migrate_with(root.path(), 8_388_608)?;
    equal(
        (
            again.already_current,
            again.rewritten_rows,
            again.dropped_rows,
        ),
        (true, 0, 0),
    )?;
    let reopened = Store::open_with(root.path(), 8_388_608)?;
    equal(Witness::capture(&reopened)?, after)
}

#[test]
fn schema1_offline_migration_retains_bound_run_bytes_without_reencoding() -> TestResult {
    schema1_fixture(Some(HISTORICAL))
}

#[test]
fn schema1_offline_migration_retains_unbound_absence_without_creating_metadata() -> TestResult {
    schema1_fixture(None)
}
