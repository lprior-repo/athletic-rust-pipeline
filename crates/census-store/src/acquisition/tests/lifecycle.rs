use super::*;

#[test]
fn explicit_capture_init_and_open_preserve_existing_store_identity() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("capture");
    let created = Store::create_capture_qualification_v1(&root)?;
    check!(eq; Store::format(&root)?.lifecycle_revision, Some(4));
    check!(matches!(
        Store::open(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    check!(matches!(
        Store::open_existing(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    check!(matches!(
        Store::inspect(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    check!(matches!(
        Store::migrate(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    check!(matches!(
        Store::installation_control_existing(&root),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    let control = Store::capture_qualification_control_existing(&root)?;
    check!(eq; control.show()?, CaptureQualification::V1(created.clone()));
    drop(control);
    let (store, writer) = Store::open_capture_qualification_v1(&root)?.into_parts();
    check!(eq; store.store_identity()?, created.store_identity().clone());
    check!(eq; writer.store_identity(), created.store_identity());
    check!(matches!(
        store.require_legacy_structured_capture(),
        Err(StoreError::Capture(
            CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    Ok(())
}

#[test]
fn capture_init_never_adopts_legacy_existing_or_artifact_roots() -> TestResult {
    for lifecycle in [2, 3] {
        let dir = tempfile::tempdir()?;
        let store = Store::open_with(dir.path(), 8_388_608)?;
        store.meta.insert(
            crate::format::LIFECYCLE_REVISION_KEY,
            lifecycle.to_string().as_bytes(),
        )?;
        let before = rows(&store)?;
        drop(store);
        check!(matches!(
            Store::create_capture_qualification_v1(dir.path()),
            Err(StoreError::Capture(CaptureError::CaptureInitExistingStore))
        ));
        let (db, _, journal, meta, receipts) = crate::open_keyspaces(dir.path(), 8_388_608)?;
        let mut after = Vec::new();
        for keyspace in [&meta, &journal, &receipts] {
            for row in db.snapshot().prefix(keyspace, b"").take(1024) {
                let (key, value) = row.into_inner()?;
                after.push((key.to_vec(), value.to_vec()));
            }
        }
        check!(eq; before, after);
    }
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("capture.body"), b"historic")?;
    check!(matches!(
        Store::create_capture_qualification_v1(dir.path()),
        Err(StoreError::Capture(CaptureError::CaptureInitRootNotEmpty))
    ));
    check!(eq; std::fs::read(dir.path().join("capture.body"))?, b"historic".to_vec());
    let empty = tempfile::tempdir()?;
    Store::create_capture_qualification_v1(empty.path())?;
    Ok(())
}

#[test]
fn capture_owner_requires_actual_bound_manifest_and_real_handle_origin() -> TestResult {
    let (dir, store, writer, held_owner) = fixture()?;
    let other = tempfile::tempdir()?;
    let other_store = Store::open_with(other.path(), 8_388_608)?;
    check!(matches!(
        writer.results_owner(&other_store, held_owner.scope().clone()),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    check!(matches!(
        held_owner.require_cache_root(&store, other.path()),
        Err(StoreError::Capture(CaptureError::CacheRootMismatch))
    ));
    let root = dir.path().join("store");
    drop(store);
    let (reopened, _) = Store::open_capture_qualification_v1(&root)?.into_parts();
    check!(matches!(
        writer.results_owner(&reopened, held_owner.scope().clone()),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    Ok(())
}

#[test]
fn capture_init_has_one_real_fjall_owner_and_missing_open_never_creates() -> TestResult {
    let dir = tempfile::tempdir()?;
    let absent = dir.path().join("absent");
    check!(Store::open_capture_qualification_v1(&absent).is_err());
    check!(!absent.exists());
    Store::create_capture_qualification_v1(&absent)?;
    let (store, _) = Store::open_capture_qualification_v1(&absent)?.into_parts();
    check!(matches!(
        Store::capture_qualification_control_existing(&absent),
        Err(StoreError::Open { .. })
    ));
    check!(matches!(
        Store::open_capture_qualification_v1(&absent),
        Err(StoreError::Open { .. })
    ));
    drop(store);
    Ok(())
}

#[test]
fn malformed_capture_migration_and_backup_never_create_missing_keyspaces_or_directories(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("partial");
    let db = fjall::Database::builder(root.join(crate::DB_DIR)).open()?;
    let meta = db.keyspace(crate::META, fjall::KeyspaceCreateOptions::default)?;
    meta.insert(crate::format::LIFECYCLE_REVISION_KEY, b"4")?;
    db.persist(fjall::PersistMode::SyncData)?;
    drop(meta);
    drop(db);
    check!(Store::migrate(&root).is_err());
    check!(Store::backup(&root, &dir.path().join("backup")).is_err());
    check!(!root.join("http").exists());
    check!(!root.join("out").exists());
    let db = fjall::Database::builder(root.join(crate::DB_DIR)).open()?;
    for name in [crate::ENTITIES, crate::JOURNAL, crate::RECEIPTS] {
        check!(!db.keyspace_exists(name));
    }
    Ok(())
}
