use super::*;

#[test]
fn missing_exact_reference_has_no_prefix_or_latest_revision_fallback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let missing = ContractRefV1::new(1, 8, reference.digest())?;
    let before = Witness::capture(&store)?;
    match store.region_contract(&missing) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::MissingContract {
            reference,
        })) => equal(reference, missing)?,
        other => return Err(format!("exact lookup used another revision: {other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn missing_payload_refuses_read_and_repeat_without_repair_or_overwrite() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let key = keys::descriptor(&reference)?;
    remove(&store, &keys::chunk(&key, 0)?)?;
    let before = Witness::capture(&store)?;
    for result in [
        store.region_contract(&reference),
        store
            .register_region_contract(&reference, &payload)
            .map(|_| payload.clone()),
    ] {
        match result {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::MissingPayload {
                    reference: missing,
                    index,
                },
            )) => {
                equal(missing, reference.clone())?;
                equal(index, 0)?;
            }
            other => return Err(format!("missing payload was repaired: {other:?}").into()),
        }
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn orphan_payload_refuses_registration_without_creating_descriptor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let key = keys::descriptor(&reference)?;
    inject(&store, &keys::chunk(&key, 0)?, b"retained orphan")?;
    let before = Witness::capture(&store)?;
    for result in [
        store.region_contract(&reference),
        store
            .register_region_contract(&reference, &payload)
            .map(|_| payload.clone()),
    ] {
        match result {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::OrphanPayload { reference: orphan },
            )) => equal(orphan, reference.clone())?,
            other => return Err(format!("orphan bytes were overwritten: {other:?}").into()),
        }
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn unexpected_extra_payload_refuses_reads_and_equivalent_repeat() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let key = keys::descriptor(&reference)?;
    inject(&store, &keys::chunk(&key, 1)?, b"extra")?;
    let before = Witness::capture(&store)?;
    for result in [
        store.region_contract(&reference),
        store
            .register_region_contract(&reference, &payload)
            .map(|_| payload.clone()),
    ] {
        match result {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::ExtraPayload {
                    reference: held,
                    index,
                    declared,
                },
            )) => {
                equal(held, reference.clone())?;
                equal((index, declared), (1, 1))?;
            }
            other => return Err(format!("extra payload was ignored: {other:?}").into()),
        }
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn malformed_chunk_key_under_exact_prefix_is_explicit_corruption() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let mut key = keys::chunk(&keys::descriptor(&reference)?, 0)?;
    key.push(0);
    inject(&store, &key, b"invalid suffix")?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::InvalidChunkKey {
            reference: held,
        })) => equal(held, reference),
        other => Err(format!("malformed chunk key was ignored: {other:?}").into()),
    }
}

#[test]
fn descriptor_reference_mismatch_refuses_read_and_preserves_retained_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let claimed = contract(8)?.reference()?;
    let mut descriptor = descriptor_value(&store, &reference)?;
    descriptor
        .as_object_mut()
        .ok_or("descriptor")?
        .insert("reference".to_string(), serde_json::to_value(&claimed)?);
    inject(
        &store,
        keys::descriptor(&reference)?.as_bytes(),
        &serde_json::to_vec(&descriptor)?,
    )?;
    let before = Witness::capture(&store)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            RegionContractRegistryError::DescriptorReferenceMismatch {
                requested,
                claimed: observed,
            },
        )) => {
            equal(requested, reference)?;
            equal(observed, claimed)?;
        }
        other => return Err(format!("spoofed descriptor accepted: {other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn truncated_and_oversized_chunks_refuse_before_payload_decode() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let bytes = wire::encode_payload(&payload)?;
    let key = keys::chunk(&keys::descriptor(&reference)?, 0)?;
    for held in [
        vec![b'x'; bytes.len().checked_sub(1).ok_or("empty payload")?],
        vec![
            b'x';
            MAX_REGION_CONTRACT_RECORD_BYTES
                .checked_add(1)
                .ok_or("chunk cap")?
        ],
    ] {
        inject(&store, &key, &held)?;
        match store.region_contract(&reference) {
            Err(StoreError::RegionContractRegistry(RegionContractRegistryError::ChunkLength {
                reference: retained,
                index,
                expected,
                observed,
            })) => {
                equal(retained, reference.clone())?;
                equal((index, expected, observed), (0, bytes.len(), held.len()))?;
            }
            other => return Err(format!("invalid chunk length reached decoder: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn changed_semantic_payload_is_immutable_conflict_and_never_replaced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let first = contract(7)?;
    let reference = first.reference()?;
    let changed = contract(8)?;
    inject_payload(&store, &reference, &wire::encode_payload(&changed)?)?;
    let before = Witness::capture(&store)?;
    for result in [
        store.region_contract(&reference),
        store
            .register_region_contract(&reference, &first)
            .map(|_| first.clone()),
    ] {
        match result {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::ImmutableConflict {
                    requested,
                    computed,
                },
            )) => {
                equal(requested, reference.clone())?;
                equal(computed, changed.reference()?)?;
            }
            other => return Err(format!("corrupted semantic payload accepted: {other:?}").into()),
        }
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn whitespace_modified_retained_payload_refuses_exact_raw_digest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let mut bytes = wire::encode_payload(&payload)?;
    bytes.push(b' ');
    inject_payload(&store, &reference, &bytes)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            RegionContractRegistryError::PayloadDigestMismatch {
                reference: held,
                computed,
            },
        )) => {
            equal(held, reference)?;
            equal(computed, crate::journal::object_digest(&bytes))
        }
        other => Err(format!("changed retained bytes accepted: {other:?}").into()),
    }
}
