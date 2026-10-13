use super::*;

type ImageEntry = (String, Option<String>, u64, String);

fn backup_image(root: &Path) -> TestResult<Vec<ImageEntry>> {
    let entries = crate::backup::manifest::digest_tree(root)?;
    ensure(entries.len() <= 256, "fixture file ceiling")?;
    Ok(entries
        .into_iter()
        .map(|entry| (entry.path, entry.target, entry.length, entry.sha256))
        .collect())
}

fn refresh_physical_manifest(root: &Path) -> TestResult {
    let path = root.join(crate::backup::MANIFEST_PATH);
    ensure(
        std::fs::metadata(&path)?.len() <= 1024 * 1024,
        "fixture manifest ceiling",
    )?;
    let mut manifest: crate::backup::Manifest = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut files = crate::backup::manifest::digest_tree(root)?;
    files.retain(|entry| entry.path != crate::backup::MANIFEST_PATH);
    manifest.files = files;
    crate::backup::manifest::write_manifest(root, &manifest)?;
    Ok(())
}

#[test]
fn empty_historical_registry_remains_compatible_without_initializing_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        let before = Witness::capture(&store)?;
        ensure(store.integrity()?.ok, "empty registry refused")?;
        equal(metadata_report(&store)?.records, 0)?;
        equal(Witness::capture(&store)?, before)?;
    }
    let backup = directory.path().join("backup");
    let restored = directory.path().join("restored");
    Store::backup(&source, &backup)?;
    Store::restore(&backup, &restored)?;
    let store = Store::open_with(&restored, CACHE_BYTES)?;
    ensure(store.integrity()?.ok, "empty restored registry refused")?;
    match store.snapshot().store_identity() {
        Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => Ok(()),
        other => Err(format!("empty restore initialized lineage: {other:?}").into()),
    }
}

#[test]
fn cold_backup_restore_preserves_exact_legacy_profile_without_lineage_requirement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let expected = populate(&source)?;
    let before = {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        Witness::capture(&store)?
    };
    let backup = directory.path().join("backup");
    let restored = directory.path().join("restored");
    Store::backup(&source, &backup)?;
    Store::restore(&backup, &restored)?;
    let store = Store::open_with(&restored, CACHE_BYTES)?;
    equal(store.profile(&expected.id()?)?, expected.clone())?;
    equal(
        store.register_profile(&expected)?,
        ProfileRegistration::Repeated(expected.id()?),
    )?;
    equal(Witness::capture(&store)?, before)?;
    ensure(store.integrity()?.ok, "valid restored profile refused")?;
    match store.snapshot().store_identity() {
        Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => Ok(()),
        other => Err(format!("legacy restore initialized lineage: {other:?}").into()),
    }
}

#[test]
fn unsupported_removed_region_metadata_is_opaque_to_profile_verifier() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    inject(&store, b"region-contracts:v1/retained-old-shape", b"broken")?;
    let before = Witness::capture(&store)?;
    let report = metadata_report(&store)?;
    equal(report.records, 0)?;
    equal(report.scope, Scope::Complete)?;
    ensure(
        report.corruptions.is_empty(),
        "profile checker reactivated region semantics",
    )?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn corrupt_profile_backup_refuses_and_preserves_previous_accepted_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let expected = populate(&source)?;
    let backup = directory.path().join("backup");
    Store::backup(&source, &backup)?;
    let accepted = backup_image(&backup)?;
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        inject(
            &store,
            crate::profiles::key(&expected.id()?).as_bytes(),
            b"{",
        )?;
    }
    match Store::backup(&source, &backup) {
        Err(StoreError::Refused { detail }) => {
            ensure(detail.contains("profile metadata"), "wrong backup refusal")?
        }
        other => return Err(format!("corrupt profile backup promoted={other:?}").into()),
    }
    equal(backup_image(&backup)?, accepted)?;
    let restored = directory.path().join("restored");
    Store::restore(&backup, &restored)?;
    let store = Store::open_with(&restored, CACHE_BYTES)?;
    equal(store.profile(&expected.id()?)?, expected)
}

#[test]
fn self_consistent_backup_file_hashes_do_not_hide_corrupt_profile_on_restore() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let expected = populate(&source)?;
    let backup = directory.path().join("backup");
    Store::backup(&source, &backup)?;
    {
        let store = Store::open_with(&backup, CACHE_BYTES)?;
        inject(
            &store,
            crate::profiles::key(&expected.id()?).as_bytes(),
            b"{",
        )?;
    }
    refresh_physical_manifest(&backup)?;
    let corrupted = backup_image(&backup)?;
    let restored = directory.path().join("restored");
    match Store::restore(&backup, &restored) {
        Err(StoreError::Refused { detail }) => {
            ensure(detail.contains("profile metadata"), "wrong restore refusal")?
        }
        other => return Err(format!("corrupt profile restored={other:?}").into()),
    }
    ensure(!restored.exists(), "refused restore was promoted")?;
    equal(backup_image(&backup)?, corrupted)
}

#[test]
fn future_profile_backup_refuses_without_replacing_previous_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let expected = populate(&source)?;
    let backup = directory.path().join("backup");
    Store::backup(&source, &backup)?;
    let accepted = backup_image(&backup)?;
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        inject(
            &store,
            crate::profiles::key(&expected.id()?).as_bytes(),
            br#"{"version":2}"#,
        )?;
    }
    match Store::backup(&source, &backup) {
        Err(StoreError::Refused { detail }) => {
            ensure(
                detail.contains("incomplete"),
                "future version called corrupt",
            )?;
            ensure(
                detail.contains("not a corruption diagnosis"),
                "unsupported disclosure absent",
            )?;
        }
        other => return Err(format!("future profile backup={other:?}").into()),
    }
    equal(backup_image(&backup)?, accepted)
}
