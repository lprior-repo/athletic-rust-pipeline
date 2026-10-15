use super::support::refused;
use super::*;

#[test]
fn public_identity_invalid_row_mark_cannot_reseed_a_missing_sequence_before_refusal() -> TestResult
{
    let (directory, store, _) = fixture()?;
    store.meta.remove("sequence:schools")?;
    store.meta.insert("rows:schools", b"bad")?;
    store.flush()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn public_identity_invalid_generation_cannot_sweep_retained_temporary_before_refusal() -> TestResult
{
    check!(!std::path::Path::new("/proc/4294967295").exists());
    let (directory, store, _) = fixture()?;
    let temporary = store.out_dir().join(".schools.jsonl.4294967295.0.part");
    std::fs::write(&temporary, b"retained before qualification refusal")?;
    store.meta.remove(crate::generation::ENTITIES)?;
    store.flush()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(eq; std::fs::read(&temporary)?, b"retained before qualification refusal");
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

#[test]
fn public_identity_preservation_guard_keeps_opaque_legacy_backup_rows_unchanged() -> TestResult {
    for key in ["schema:store_version", "schema:lifecycle_revision"] {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("legacy");
        let store = Store::open_with(&root, 8_388_608)?;
        store.store_identity()?;
        store.append(Table::Athletes, &subject("legacy-backup", "909")?)?;
        store.meta.insert(key, b"historical-opaque")?;
        store.flush()?;
        let before = retained_state(&store)?;
        drop(store);
        let backup = directory.path().join("backup");
        Store::backup(&root, &backup)?;
        check!(eq; physical_state(&root)?, before);
        let restored = directory.path().join("restored");
        Store::restore(&backup, &restored)?;
        check!(eq; physical_state(&restored)?, before);
        check!(eq; physical_state(&root)?, before);
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum FutureFamily {
    Metadata,
    Journal,
    Receipt,
}

const FUTURE_FAMILIES: [FutureFamily; 3] = [
    FutureFamily::Metadata,
    FutureFamily::Journal,
    FutureFamily::Receipt,
];

fn insert_future_orphan(store: &Store, family: FutureFamily) -> TestResult {
    match family {
        FutureFamily::Metadata => {
            store
                .meta
                .insert("public_identity:v2:qualification", b"future metadata")?;
        }
        FutureFamily::Journal => {
            store.journal.insert(
                Store::journal_key("public_identity:v2", "orphan"),
                b"future journal",
            )?;
        }
        FutureFamily::Receipt => {
            store
                .receipts
                .insert("public_identity_v2:orphan", b"future receipt")?;
        }
    }
    store.flush()?;
    Ok(())
}

#[test]
fn public_identity_future_orphans_block_legacy_open_and_migration_without_mutation() -> TestResult {
    for family in FUTURE_FAMILIES {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("legacy");
        let store = Store::open_with(&root, 8_388_608)?;
        check!(eq; store.store_format()?.lifecycle_revision, Some(2));
        insert_future_orphan(&store, family)?;
        let before = retained_state(&store)?;
        drop(store);
        check!(matches!(
            refused(Store::open(&root))?,
            StoreError::PublicIdentity(crate::PublicIdentityError::OptInRequired)
        ));
        check!(eq; physical_state(&root)?, before);
        check!(matches!(
            refused(Store::migrate(&root))?,
            StoreError::PublicIdentity(crate::PublicIdentityError::OptInRequired)
        ));
        check!(eq; physical_state(&root)?, before);
    }
    Ok(())
}

#[test]
fn public_identity_future_orphans_block_qualification_reopen_without_mutation() -> TestResult {
    for family in FUTURE_FAMILIES {
        let (directory, store, _) = fixture()?;
        check!(eq; store.store_format()?.lifecycle_revision, Some(5));
        insert_future_orphan(&store, family)?;
        let before = retained_state(&store)?;
        drop(store);
        let root = directory.path().join("identity");
        check!(matches!(
            refused(Store::open_public_identity_qualification_v1(&root))?,
            StoreError::PublicIdentity(crate::PublicIdentityError::Corrupt(_))
        ));
        check!(eq; physical_state(&root)?, before);
    }
    Ok(())
}
