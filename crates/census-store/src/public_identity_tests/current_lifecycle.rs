use census_domain::public_identity_history::genesis;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

use super::*;

#[test]
fn public_identity_current_initialization_and_reopen_retain_one_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    for root in [
        directory.path().join("absent"),
        directory.path().join("empty"),
    ] {
        if root.ends_with("empty") {
            std::fs::create_dir(&root)?;
        }
        let declaration = Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        check!(eq; store.store_format()?.lifecycle_revision, Some(7));
        check!(eq; store.store_identity()?, declaration.store_identity().clone());
        check!(eq; writer.store_identity(), declaration.store_identity());
        check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
        check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
        let before = retained_state(&store)?;
        drop(store);
        let (reopened, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        check!(eq; reopened.store_identity()?, declaration.store_identity().clone());
        check!(eq; writer.head(&reopened)?, genesis(declaration.store_identity())?);
        check!(eq; retained_state(&reopened)?, before);
    }
    Ok(())
}

#[test]
fn public_identity_current_missing_open_never_creates_and_second_owner_refuses() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    check!(Store::open_public_identity_qualification_v3(&root).is_err());
    check!(!root.exists());
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, _) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let before = retained_state(&store)?;
    check!(matches!(
        Store::open_public_identity_qualification_v3(&root),
        Err(StoreError::Open { .. })
    ));
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_current_partial_store_refusal_never_creates_keyspaces_or_output_dirs(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("partial");
    let db = fjall::Database::builder(root.join(crate::DB_DIR)).open()?;
    let meta = db.keyspace(crate::META, fjall::KeyspaceCreateOptions::default)?;
    meta.insert(crate::format::LIFECYCLE_REVISION_KEY, b"7")?;
    db.persist(fjall::PersistMode::SyncData)?;
    drop(meta);
    drop(db);
    check!(Store::open_public_identity_qualification_v3(&root).is_err());
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
