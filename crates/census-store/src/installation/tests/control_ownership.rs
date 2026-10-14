use super::*;

#[test]
fn continuous_owner_lock_blocks_second_control_and_second_generic_open() -> TestResult {
    let root = tempfile::tempdir()?;
    let _config = prepare(root.path())?;
    let store = Store::open_existing(root.path())?;
    if !matches!(
        Store::installation_control_existing(root.path()),
        Err(StoreError::Open { .. })
    ) {
        return Err("control coexisted with owner".into());
    }
    drop(store);
    let control = Store::installation_control_existing(root.path())?;
    if !matches!(
        Store::open_existing(root.path()),
        Err(StoreError::Open { .. })
    ) {
        return Err("owner coexisted with control".into());
    }
    if !matches!(
        Store::installation_control_existing(root.path()),
        Err(StoreError::Open { .. })
    ) {
        return Err("two controls coexisted".into());
    }
    drop(control);
    Store::open_existing(root.path())?;
    Ok(())
}

#[test]
fn shared_internal_store_custody_is_rejected_defensively() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let store = Store::open_existing(root.path())?;
    let other = store.clone();
    let mut control = InstallationControl::new(store);
    let before = witness(&control.store)?;
    if !matches!(
        control.enroll(&config),
        Err(StoreError::Installation(
            InstallationError::HistoricalStatePresent {
                kind: HistoricalStateKind::SharedOwner
            }
        ))
    ) {
        return Err("shared custody accepted".into());
    }
    equal(witness(&control.store)?, before.clone())?;
    drop(other);
    control.enroll(&config)?;
    Ok(())
}

#[test]
fn control_open_refuses_missing_store_without_creating_root_or_census_keyspaces() -> TestResult {
    let dir = tempfile::tempdir()?;
    let absent = dir.path().join("absent");
    if !matches!(
        Store::installation_control_existing(&absent),
        Err(StoreError::Io { .. })
    ) {
        return Err("missing control root accepted".into());
    }
    equal(absent.exists(), false)?;
    let bare = dir.path().join("bare");
    let db = fjall::Database::builder(bare.join("fjall")).open()?;
    drop(db);
    if !matches!(
        Store::installation_control_existing(&bare),
        Err(StoreError::Refused { .. })
    ) {
        return Err("bare database accepted".into());
    }
    let db = fjall::Database::builder(bare.join("fjall")).open()?;
    equal(db.keyspace_count(), 0)?;
    equal(bare.join("http").exists(), false)?;
    equal(bare.join("out").exists(), false)
}

#[test]
fn missing_row_mark_is_never_defaulted_or_seeded_by_control() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let store = Store::open_existing(root.path())?;
    let key = format!("rows:{}", Table::Schools.file());
    store.meta.remove(&key)?;
    store.flush()?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    if !matches!(
        control.enroll(&config),
        Err(StoreError::Installation(
            InstallationError::HistoricalStatePresent {
                kind: HistoricalStateKind::Metadata
            }
        ))
    ) {
        return Err("missing mark accepted".into());
    }
    equal(control.store.meta.contains_key(&key)?, false)?;
    equal(witness(&control.store)?, before.clone())
}
