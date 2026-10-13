use super::*;

#[test]
fn pinned_snapshot_refuses_later_registration_without_live_fallback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let identity = store.store_identity()?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let view = store.snapshot();
    let sequence = view.sequence();
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    for result in [
        view.region_contract(&reference),
        view.selected_region_contract(&identity, &reference),
    ] {
        match result {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::MissingContract { reference: missing },
            )) => equal(missing, reference.clone())?,
            other => return Err(format!("snapshot saw later live bytes: {other:?}").into()),
        }
    }
    equal(view.sequence(), sequence)?;
    equal(
        store.selected_region_contract(&identity, &reference)?,
        payload,
    )
}

#[test]
fn pinned_payload_and_identity_survive_later_live_corruption() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let identity = store.store_identity()?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let view = store.snapshot();
    let key = keys::descriptor(&reference)?;
    remove(&store, &keys::chunk(&key, 0)?)?;
    equal(
        view.selected_region_contract(&identity, &reference)?,
        payload,
    )?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::MissingPayload {
            reference: missing,
            index,
        })) => {
            equal(missing, reference)?;
            equal(index, 0)
        }
        other => Err(format!("unexpected live read: {other:?}").into()),
    }
}

#[test]
fn same_contract_in_independent_stores_refuses_foreign_selected_identity() -> TestResult {
    let left_dir = tempfile::tempdir()?;
    let right_dir = tempfile::tempdir()?;
    let left = Store::open_with(left_dir.path(), 8_388_608)?;
    let right = Store::open_with(right_dir.path(), 8_388_608)?;
    let left_id = left.store_identity()?;
    let right_id = right.store_identity()?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    for store in [&left, &right] {
        equal(
            store.register_region_contract(&reference, &payload)?,
            RegionContractRegistration::Registered(reference.clone()),
        )?;
    }
    let before = Witness::capture(&right)?;
    for result in [
        right.selected_region_contract(&left_id, &reference),
        right
            .snapshot()
            .selected_region_contract(&left_id, &reference),
    ] {
        match result {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::ForeignStore { expected, actual },
            )) => {
                equal(expected, left_id.clone())?;
                equal(actual, right_id.clone())?;
            }
            other => return Err(format!("foreign identity accepted: {other:?}").into()),
        }
    }
    equal(Witness::capture(&right)?, before)
}

#[test]
fn selected_read_of_legacy_unbound_store_never_mints_lineage() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let identity = StoreIdentity::parse(&"A".repeat(64))?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let before = Witness::capture(&store)?;
    for result in [
        store.selected_region_contract(&identity, &reference),
        store
            .snapshot()
            .selected_region_contract(&identity, &reference),
    ] {
        match result {
            Err(StoreError::StoreLineage(crate::StoreLineageError::Missing)) => {}
            other => return Err(format!("legacy lineage was fabricated: {other:?}").into()),
        }
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn pinned_read_keeps_missing_chunk_missing_after_later_live_restoration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let bytes = wire::encode_payload(&payload)?;
    let key = keys::descriptor(&reference)?;
    let chunk = keys::chunk(&key, 0)?;
    remove(&store, &chunk)?;
    let view = store.snapshot();
    inject(&store, &chunk, &bytes)?;
    match view.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::MissingPayload {
            reference: missing,
            index,
        })) => {
            equal(missing, reference.clone())?;
            equal(index, 0)?;
        }
        other => return Err(format!("pinned gap used live restoration: {other:?}").into()),
    }
    equal(store.region_contract(&reference)?, payload)
}
