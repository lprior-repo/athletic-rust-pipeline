use crate::{Store, StoreError};
use fjall::{Database, KeyspaceCreateOptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn existing_open_refuses_missing_paths_without_creation() -> TestResult {
    let root = tempfile::tempdir()?;
    let absent = root.path().join("absent");
    check!(matches!(
        Store::open_existing(&absent),
        Err(StoreError::Io { .. })
    ));
    check!(!absent.exists());
    Ok(())
}

#[test]
fn existing_open_refuses_bare_fjall_without_creating_census_keyspaces() -> TestResult {
    let root = tempfile::tempdir()?;
    let db = Database::builder(root.path().join("fjall")).open()?;
    check!(eq; db.keyspace_count(), 0);
    drop(db);
    check!(matches!(
        Store::open_existing(root.path()),
        Err(StoreError::Refused { .. })
    ));
    let db = Database::builder(root.path().join("fjall")).open()?;
    check!(eq; db.keyspace_count(), 0);
    check!(!root.path().join("http").exists());
    check!(!root.path().join("out").exists());
    Ok(())
}

#[test]
fn existing_open_refuses_empty_census_keyspaces_without_initialization() -> TestResult {
    let root = tempfile::tempdir()?;
    let db = Database::builder(root.path().join("fjall")).open()?;
    for name in [
        crate::ENTITIES,
        crate::JOURNAL,
        crate::META,
        crate::RECEIPTS,
    ] {
        db.keyspace(name, KeyspaceCreateOptions::default)?;
    }
    drop(db);
    check!(matches!(
        Store::open_existing(root.path()),
        Err(StoreError::MigrationRequired { .. })
    ));
    let db = Database::builder(root.path().join("fjall")).open()?;
    let meta = db.keyspace(crate::META, KeyspaceCreateOptions::default)?;
    check!(meta.is_empty()?);
    check!(!root.path().join("out").exists());
    Ok(())
}

#[test]
fn existing_open_preserves_current_rows_and_refuses_future_schema() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    store.journal_done(
        "capture",
        "retained",
        &serde_json::json!({"raw":"original"}),
    )?;
    let expected = store.journal_payload("capture", "retained")?;
    let sequence = store.snapshot().sequence();
    drop(store);
    let store = Store::open_existing(root.path())?;
    check!(eq; store.journal_payload("capture", "retained")?, expected);
    check!(eq; store.snapshot().sequence(), sequence);
    store.meta.insert("schema:store_version", "999")?;
    store.flush()?;
    drop(store);
    check!(matches!(
        Store::open_existing(root.path()),
        Err(StoreError::FormatNewer { .. })
    ));
    let (_, _, journal, meta, _) = crate::open_keyspaces(root.path(), 8 * 1024 * 1024)?;
    check!(eq; crate::meta::get_u32(&meta, "schema:store_version")?, Some(999));
    check!(journal.contains_key(Store::journal_key("capture", "retained"))?);
    Ok(())
}
