use super::support::refused;
use crate::PublicIdentityError;
use census_domain::public_identity_history::genesis;

use super::*;

#[test]
fn public_identity_initialization_and_reopen_retain_one_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    for root in [
        directory.path().join("absent"),
        directory.path().join("empty"),
    ] {
        if root.ends_with("empty") {
            std::fs::create_dir(&root)?;
        }
        let declaration = Store::create_public_identity_qualification_v1(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
        check!(eq; store.store_format()?.lifecycle_revision, Some(5));
        check!(eq; store.store_identity()?, declaration.store_identity().clone());
        check!(eq; writer.store_identity(), declaration.store_identity());
        check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
        let before = retained_state(&store)?;
        drop(store);
        let (reopened, writer) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
        check!(eq; reopened.store_identity()?, declaration.store_identity().clone());
        check!(eq; writer.head(&reopened)?, genesis(declaration.store_identity())?);
        check!(eq; retained_state(&reopened)?, before);
    }
    Ok(())
}

#[test]
fn public_identity_generic_openers_refuse_without_changing_retained_rows() -> TestResult {
    let (directory, store, _) = fixture()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    check!(matches!(
        refused(Store::open(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    check!(matches!(
        refused(Store::open_existing(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    check!(matches!(
        refused(Store::inspect(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    check!(matches!(
        refused(Store::migrate(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    check!(matches!(
        refused(Store::installation_control_existing(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    check!(matches!(
        refused(Store::capture_qualification_control_existing(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    check!(matches!(
        refused(Store::open_capture_qualification_v1(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(matches!(
        refused(Store::open_capture_qualification_v2(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

#[test]
fn public_identity_initialization_never_adopts_an_existing_store_or_artifact() -> TestResult {
    let legacy = tempfile::tempdir()?;
    let store = Store::open_with(legacy.path(), 8_388_608)?;
    store.store_identity()?;
    let before = retained_state(&store)?;
    drop(store);
    check!(matches!(
        refused(Store::create_public_identity_qualification_v1(
            legacy.path()
        ))?,
        StoreError::PublicIdentity(PublicIdentityError::ExistingRoot)
    ));
    check!(eq; physical_state(legacy.path())?, before);
    let (directory, store, _) = fixture()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    check!(matches!(
        refused(Store::create_public_identity_qualification_v1(&root))?,
        StoreError::PublicIdentity(PublicIdentityError::ExistingRoot)
    ));
    check!(eq; physical_state(&root)?, before);
    let artifact = tempfile::tempdir()?;
    let path = artifact.path().join("historical.body");
    std::fs::write(&path, b"retained historical evidence")?;
    check!(matches!(
        refused(Store::create_public_identity_qualification_v1(
            artifact.path()
        ))?,
        StoreError::PublicIdentity(PublicIdentityError::ExistingRoot)
    ));
    check!(eq; std::fs::read(&path)?, b"retained historical evidence");
    check!(eq; std::fs::read_dir(artifact.path())?.count(), 1);
    Ok(())
}

#[test]
fn public_identity_missing_open_never_creates_and_second_owner_refuses() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(!root.exists());
    Store::create_public_identity_qualification_v1(&root)?;
    let (store, _) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
    check!(matches!(
        Store::open_public_identity_qualification_v1(&root),
        Err(StoreError::Open { .. })
    ));
    check!(eq; store.store_format()?.lifecycle_revision, Some(5));
    Ok(())
}

#[test]
fn public_identity_missing_lineage_is_refused_and_never_reminted() -> TestResult {
    let (directory, store, writer) = fixture()?;
    let key = Store::journal_key("export-lineage-v1", "store-identity");
    store.journal.remove(&key)?;
    store.flush()?;
    let before = retained_state(&store)?;
    check!(matches!(
        store.store_identity(),
        Err(StoreError::StoreLineage(_))
    ));
    check!(writer.head(&store).is_err());
    check!(eq; retained_state(&store)?, before);
    check!(!store.journal.contains_key(&key)?);
    drop(store);
    let root = directory.path().join("identity");
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

#[test]
fn public_identity_partial_store_refusal_never_creates_keyspaces_or_output_dirs() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("partial");
    let db = fjall::Database::builder(root.join(crate::DB_DIR)).open()?;
    let meta = db.keyspace(crate::META, fjall::KeyspaceCreateOptions::default)?;
    meta.insert(crate::format::LIFECYCLE_REVISION_KEY, b"5")?;
    db.persist(fjall::PersistMode::SyncData)?;
    drop(meta);
    drop(db);
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(Store::open(&root).is_err());
    check!(Store::inspect(&root).is_err());
    check!(Store::migrate(&root).is_err());
    check!(!root.join("http").exists());
    check!(!root.join("out").exists());
    let db = fjall::Database::builder(root.join(crate::DB_DIR)).open()?;
    for name in [crate::ENTITIES, crate::JOURNAL, crate::RECEIPTS] {
        check!(!db.keyspace_exists(name));
    }
    Ok(())
}
