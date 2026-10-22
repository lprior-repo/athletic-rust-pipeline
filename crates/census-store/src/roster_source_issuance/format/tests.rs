use super::*;
use crate::roster_source_issuance::RosterStartupValidationBudgetV1;
use fjall::KeyspaceCreateOptions;
use tempfile::tempdir;

fn temp_table() -> Result<
    (tempfile::TempDir, fjall::Database, fjall::Keyspace),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let directory = tempdir()?;
    let db = fjall::Database::builder(directory.path().join("db"))
        .open()
        .map_err(|source| format!("db open: {source}"))?;
    let table = db
        .keyspace("roster_test", KeyspaceCreateOptions::default)
        .map_err(|source| format!("keyspace: {source}"))?;
    Ok((directory, db, table))
}

fn insert_rows(
    db: &fjall::Database,
    table: &fjall::Keyspace,
    prefix: &str,
    count: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut b = db.batch();
    for index in 0..count {
        let key = format!("{prefix}row-{index:04}");
        b.insert(table, key.as_bytes(), b"v");
    }
    b.commit().map_err(|source| format!("commit: {source}"))?;
    Ok(())
}

fn insert_row(
    table: &fjall::Keyspace,
    key: &str,
    value: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    table
        .insert(key.as_bytes(), value)
        .map_err(|source| format!("insert: {source}"))?;
    Ok(())
}

#[test]
fn combined_prefixes_admit_exact_configured_row_budget(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_dir, db, table) = temp_table()?;
    insert_rows(&db, &table, super::super::ROOT, 2)?;
    insert_rows(&db, &table, "roster_source_issuance\0", 2)?;

    let snapshot = db.snapshot();
    let mut budget = RosterStartupValidationBudgetV1::for_test(4, 16 * 1024);
    let rows = protected_rows(&snapshot, &table, &mut budget, None)?;
    check!(eq; rows.len(), 4);
    Ok(())
}

#[test]
fn combined_prefixes_refuse_one_more_than_configured_row_budget(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_dir, db, table) = temp_table()?;
    insert_rows(&db, &table, super::super::ROOT, 2)?;
    insert_rows(&db, &table, "roster_source_issuance\0", 3)?;

    let snapshot = db.snapshot();
    let mut budget = RosterStartupValidationBudgetV1::for_test(4, 16 * 1024);
    let result = protected_rows(&snapshot, &table, &mut budget, None);
    check!(matches!(
        result,
        Err(StoreError::RosterSourceIssuance(Error::BudgetExceeded))
    ));
    Ok(())
}

#[test]
fn bare_key_consumes_the_same_configured_row_budget(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_dir, db, table) = temp_table()?;
    insert_rows(&db, &table, super::super::ROOT, 4)?;
    insert_row(&table, "roster_source_issuance", b"extra")?;

    let snapshot = db.snapshot();
    let mut budget = RosterStartupValidationBudgetV1::for_test(4, 16 * 1024);
    let result = protected_rows(&snapshot, &table, &mut budget, None);
    check!(matches!(
        result,
        Err(StoreError::RosterSourceIssuance(Error::BudgetExceeded))
    ));
    Ok(())
}

#[test]
fn startup_refuses_bare_reserved_state_without_grant_or_enrollment(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_directory, db, metadata) = temp_table()?;
    let journal = db.keyspace("journal", KeyspaceCreateOptions::default)?;
    let receipts = db.keyspace("receipts", KeyspaceCreateOptions::default)?;
    insert_row(&metadata, "roster_source_issuance", b"unadmitted")?;
    let format = StoreFormat {
        schema_version: None,
        key_format: None,
        lifecycle_revision: None,
        created_by: None,
        created_at: None,
        migration_target: None,
    };
    let result = validated_lifecycle(
        &db.snapshot(),
        &metadata,
        &journal,
        &receipts,
        &format,
        &mut RosterStartupValidationBudgetV1::new(),
    );
    check!(matches!(
        result,
        Err(StoreError::RosterSourceIssuance(Error::Corrupt(_)))
    ));
    Ok(())
}

#[test]
fn combined_prefixes_plus_bare_key_fit_exact_configured_row_budget(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (_directory, db, table) = temp_table()?;
    insert_rows(&db, &table, super::super::ROOT, 2)?;
    insert_rows(&db, &table, "roster_source_issuance\0", 1)?;
    insert_row(&table, "roster_source_issuance", b"extra")?;
    let rows = protected_rows(
        &db.snapshot(),
        &table,
        &mut RosterStartupValidationBudgetV1::for_test(4, 16 * 1024),
        None,
    )?;
    check!(eq; rows.len(), 4);
    check!(eq; rows.last().map(|(key, value)| (key.as_slice(), value.as_slice())),
        Some((b"roster_source_issuance".as_slice(), b"extra".as_slice())));
    Ok(())
}
