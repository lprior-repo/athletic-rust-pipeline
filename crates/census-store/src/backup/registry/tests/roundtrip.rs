use super::*;

pub(super) fn assert_restored(store: &Store, fixture: &Fixture) -> TestResult {
    equal(store.snapshot().store_identity()?, fixture.identity.clone())?;
    equal(store.store_identity()?, fixture.identity.clone())?;
    for profile in &fixture.profiles {
        let selection = ProfileSelectionV1::new(fixture.identity.clone(), profile.id()?);
        equal(store.profile(&profile.id()?)?, profile.clone())?;
        equal(
            store.snapshot().selected_profile(&selection)?,
            profile.clone(),
        )?;
        equal(store.selected_profile(&selection)?, profile.clone())?;
    }
    for region in &fixture.regions {
        let reference = region.reference()?;
        equal(store.region_contract(&reference)?, region.clone())?;
        equal(
            store
                .snapshot()
                .selected_region_contract(&fixture.identity, &reference)?,
            region.clone(),
        )?;
        equal(
            store.selected_region_contract(&fixture.identity, &reference)?,
            region.clone(),
        )?;
    }
    equal(
        std::fs::read(store.root().join("http/retained.bytes"))?,
        b"immutable captured source bytes".to_vec(),
    )?;
    equal(
        std::fs::read(store.root().join("out/historical-artifact.json"))?,
        b"{\"historical\":true}".to_vec(),
    )?;
    equal(&Witness::capture(store)?, &fixture.witness)?;
    ensure(
        store.integrity()?.ok,
        "registered restored metadata must pass generic retained integrity",
    )
}

#[test]
fn full_cold_backup_restore_reopen_preserves_exact_profile_region_and_uppercase_lineage(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source");
    let backup = dir.path().join("backup");
    let restored = dir.path().join("restored");
    let fixture = populate(&source, None)?;
    let report = Store::backup(&source, &backup)?;
    equal(report.tables.get("schools").copied(), Some(1))?;
    let copied = Store::restore(&backup, &restored)?;
    equal(copied.tables, report.tables)?;
    {
        let store = Store::open_with(&restored, CACHE_BYTES)?;
        assert_restored(&store, &fixture)?;
        let before = Witness::capture(&store)?;
        for profile in &fixture.profiles {
            equal(
                store.register_profile(profile)?,
                ProfileRegistration::Repeated(profile.id()?),
            )?;
        }
        for region in &fixture.regions {
            equal(
                store.register_region_contract(&region.reference()?, region)?,
                RegionContractRegistration::Repeated(region.reference()?),
            )?;
        }
        equal(Witness::capture(&store)?, before)?;
    }
    let reopened = Store::open_with(&restored, CACHE_BYTES)?;
    assert_restored(&reopened, &fixture)?;
    let original = Store::open_with(&source, CACHE_BYTES)?;
    assert_restored(&original, &fixture)
}

#[test]
fn independently_initialized_equivalent_registry_refuses_original_installation_selections(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let original = populate(&dir.path().join("original"), None)?;
    let independent = populate(
        &dir.path().join("independent"),
        Some(StoreIdentity::parse(&"B".repeat(64))?),
    )?;
    equal(&original.profiles, &independent.profiles)?;
    equal(&original.regions, &independent.regions)?;
    ensure(
        original.identity != independent.identity,
        "independent fixture must have distinct lineage",
    )?;
    let store = Store::open_with(dir.path().join("independent"), CACHE_BYTES)?;
    let before = Witness::capture(&store)?;
    let profile = independent.profiles.first().ok_or("profile")?;
    let selection = ProfileSelectionV1::new(original.identity.clone(), profile.id()?);
    match store.selected_profile(&selection) {
        Err(StoreError::ProfileRegistry(crate::ProfileRegistryError::Selection(
            ProfileSelectionError::ForeignStore,
        ))) => {}
        other => return Err(format!("foreign profile selection={other:?}").into()),
    }
    let region = independent.regions.first().ok_or("region")?;
    match store.selected_region_contract(&original.identity, &region.reference()?) {
        Err(StoreError::RegionContractRegistry(
            crate::RegionContractRegistryError::ForeignStore { expected, actual },
        )) => {
            equal(expected, original.identity)?;
            equal(actual, independent.identity)?;
        }
        other => return Err(format!("foreign region selection={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn full_restore_does_not_create_missing_historical_lineage() -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("legacy");
    let backup = dir.path().join("backup");
    let restored = dir.path().join("restored");
    let region = region(1, 0)?;
    let profile = profile("2026-10-11", region.reference()?)?;
    {
        let store = Store::open_with(&source, CACHE_BYTES)?;
        store.register_profile(&profile)?;
        store.register_region_contract(&region.reference()?, &region)?;
        store.flush()?;
    }
    Store::backup(&source, &backup)?;
    Store::restore(&backup, &restored)?;
    for _ in 0..2 {
        let store = Store::open_with(&restored, CACHE_BYTES)?;
        let before = Witness::capture(&store)?;
        match store.snapshot().store_identity() {
            Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => {}
            other => return Err(format!("restore initialized absent lineage={other:?}").into()),
        }
        let identity = StoreIdentity::parse(&"A".repeat(64))?;
        let selection = ProfileSelectionV1::new(identity.clone(), profile.id()?);
        match store.selected_profile(&selection) {
            Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => {}
            other => return Err(format!("selected profile created lineage={other:?}").into()),
        }
        match store.selected_region_contract(&identity, &region.reference()?) {
            Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => {}
            other => return Err(format!("selected region created lineage={other:?}").into()),
        }
        equal(store.profile(&profile.id()?)?, profile.clone())?;
        equal(store.region_contract(&region.reference()?)?, region.clone())?;
        ensure(
            store.integrity()?.ok,
            "legacy retained metadata can be healthy with no required selection",
        )?;
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}
