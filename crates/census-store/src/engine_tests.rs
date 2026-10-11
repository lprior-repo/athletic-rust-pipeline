use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

fn operation() -> EngineOperation {
    EngineOperation::source_sweep("wiaa", "wi", "2026-outdoor", 1).unwrap()
}

fn digest() -> EngineDigest {
    EngineDigest::new("engine-digest-a".to_string()).unwrap()
}

fn rows(store: &Store, table: Table) -> u64 {
    store
        .stats()
        .unwrap()
        .tables
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| count)
        .unwrap()
}

#[test]
fn engine_commits_observations_progress_and_receipt_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let outcome = store
        .engine_commit(&operation(), &digest(), |batch| {
            batch.append_many(Table::Schools, &[school("Abbotsford"), school("Colby")])?;
            batch.journal_done("wiaa_schools", "unit-0", &"engine-page")?;
            Ok(())
        })
        .unwrap();
    assert!(outcome.written());
    assert_eq!(outcome.appended(), 2);
    assert_eq!(rows(&store, Table::Schools), 2);
    assert!(store
        .journal_keys("wiaa_schools")
        .unwrap()
        .contains("unit-0"));
    assert!(store.receipt(operation().as_str()).unwrap().is_some());
}

#[test]
fn engine_replay_of_one_operation_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let stage = |batch: &mut StoreBatch| {
        batch.append_many(Table::Schools, &[school("Abbotsford")])?;
        batch.journal_done("wiaa_schools", "unit-0", &"engine-page")?;
        Ok(())
    };
    let first = store.engine_commit(&operation(), &digest(), stage).unwrap();
    assert!(first.written());
    let repeat = store.engine_commit(&operation(), &digest(), stage).unwrap();
    assert!(repeat.repeated());
    assert_eq!(repeat.appended(), 0);
    assert_eq!(rows(&store, Table::Schools), 1);
}

#[test]
fn engine_refuses_a_second_payload_under_one_operation() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .engine_commit(&operation(), &digest(), |batch| {
            batch.append_many(Table::Schools, &[school("Abbotsford")])?;
            Ok(())
        })
        .unwrap();
    let other = EngineDigest::new("engine-digest-b".to_string()).unwrap();
    let refused = store.engine_commit(&operation(), &other, |batch| {
        batch.append_many(Table::Schools, &[school("Colby")])?;
        Ok(())
    });
    assert!(matches!(refused, Err(StoreError::Invariant { .. })));
    assert_eq!(rows(&store, Table::Schools), 1);
}

#[test]
fn engine_refuses_operations_without_a_stable_scope() {
    assert!(EngineOperation::new("".to_string()).is_err());
    assert!(EngineOperation::new("bare-operation".to_string()).is_err());
    assert!(EngineOperation::jurisdiction("wi", "2026-outdoor", 0).is_err());
    assert!(EngineDigest::new("".to_string()).is_err());
    let first = EngineOperation::jurisdiction("wi", "2026-outdoor", 1).unwrap();
    let repeat = EngineOperation::jurisdiction("wi", "2026-outdoor", 1).unwrap();
    assert_eq!(first, repeat);
    assert_eq!(first.as_str(), "jurisdiction:wi:2026-outdoor:1");
}

#[test]
fn engine_staging_failure_writes_neither_rows_nor_receipts() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let refused: StoreResult<Application> = store.engine_commit(&operation(), &digest(), |_| {
        Err(StoreError::Refused {
            detail: "staged page failed validation".to_string(),
        })
    });
    assert!(matches!(refused, Err(StoreError::Refused { .. })));
    assert_eq!(rows(&store, Table::Schools), 0);
    assert!(store.receipt(operation().as_str()).unwrap().is_none());
}
