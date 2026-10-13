use super::*;

#[test]
fn first_registration_changes_only_its_exact_descriptor_and_chunk_keys() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    inject(
        &store,
        b"unrelated-registry:v1/retained",
        b"historical metadata",
    )?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let descriptor = keys::descriptor(&reference)?.into_bytes();
    let chunk = keys::chunk(&keys::descriptor(&reference)?, 0)?;
    let before = Witness::capture(&store)?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let after = Witness::capture(&store)?;
    before.unchanged_evidence(&after)?;
    equal(
        after.meta.len(),
        before.meta.len().checked_add(2).ok_or("metadata count")?,
    )?;
    let mut retained = Vec::new();
    retained.try_reserve_exact(before.meta.len())?;
    let mut inserted = 0usize;
    for (key, value) in after.meta {
        if key == descriptor || key == chunk {
            inserted = inserted.checked_add(1).ok_or("insert count")?;
        } else {
            retained.push((key, value));
        }
    }
    equal(inserted, 2)?;
    equal(retained, before.meta)
}

#[test]
fn snapshot_before_identity_creation_keeps_selected_region_unbound() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let view = store.snapshot();
    let identity = store.store_identity()?;
    let before = Witness::capture(&store)?;
    match view.selected_region_contract(&identity, &reference) {
        Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => {}
        other => return Err(format!("pinned selection adopted later identity: {other:?}").into()),
    }
    equal(view.region_contract(&reference)?, payload.clone())?;
    equal(
        store.selected_region_contract(&identity, &reference)?,
        payload,
    )?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn pinned_multi_chunk_lookup_retains_original_descriptor_and_every_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let identity = store.store_identity()?;
    let payload = RegionMembershipContractV1::new(sized_input(65_537)?)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let view = store.snapshot();
    let key = keys::descriptor(&reference)?;
    let original = store.meta.get(key.as_bytes())?.ok_or("descriptor")?;
    inject(
        &store,
        key.as_bytes(),
        br#"{"version":2,"future_shape":true}"#,
    )?;
    inject(&store, &keys::chunk(&key, 0)?, &vec![b'x'; 65_536])?;
    remove(&store, &keys::chunk(&key, 1)?)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            RegionContractRegistryError::UnsupportedVersion { what, found },
        )) => equal((what, found), ("region descriptor", 2))?,
        other => return Err(format!("live corruption was hidden: {other:?}").into()),
    }
    equal(
        view.selected_region_contract(&identity, &reference)?,
        payload.clone(),
    )?;
    inject(&store, key.as_bytes(), original.as_ref())?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::MissingPayload {
            reference: missing,
            index,
        })) => {
            equal(missing, reference.clone())?;
            equal(index, 1)?;
        }
        other => return Err(format!("live chunk gap was hidden: {other:?}").into()),
    }
    equal(
        view.selected_region_contract(&identity, &reference)?,
        payload,
    )
}
