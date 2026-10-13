use super::*;

#[test]
fn exact_one_mib_payload_uses_sixteen_bounded_records_and_reopens() -> TestResult {
    let dir = tempfile::tempdir()?;
    let payload = RegionMembershipContractV1::new(sized_input(MAX_REGION_PAYLOAD_BYTES)?)?;
    let reference = payload.reference()?;
    {
        let store = Store::open_with(dir.path(), 8_388_608)?;
        equal(
            wire::encode_payload(&payload)?.len(),
            MAX_REGION_PAYLOAD_BYTES,
        )?;
        equal(
            store.register_region_contract(&reference, &payload)?,
            RegionContractRegistration::Registered(reference.clone()),
        )?;
        equal(store.region_contract(&reference)?, payload.clone())?;
        let key = keys::descriptor(&reference)?;
        let raw = store.meta.get(key.as_bytes())?.ok_or("descriptor")?;
        let descriptor: wire::Descriptor = serde_json::from_slice(raw.as_ref())?;
        equal(
            (
                descriptor.payload_bytes,
                descriptor.chunk_bytes,
                descriptor.chunks,
            ),
            (1_048_576, 65_536, 16),
        )?;
        equal(raw.len() < MAX_REGION_CONTRACT_RECORD_BYTES, true)?;
        for index in 0..16 {
            let bytes = store.meta.get(keys::chunk(&key, index)?)?.ok_or("chunk")?;
            equal(bytes.len(), 65_536)?;
        }
    }
    let reopened = Store::open_with(dir.path(), 8_388_608)?;
    equal(reopened.region_contract(&reference)?, payload)
}

#[test]
fn one_mib_plus_one_domain_payload_is_refused_without_waiving_existing_cap() -> TestResult {
    let target = MAX_REGION_PAYLOAD_BYTES.checked_add(1).ok_or("byte cap")?;
    match RegionMembershipContractV1::new(sized_input(target)?) {
        Err(census_domain::model::RegionMembershipError::PayloadTooLarge { maximum }) => {
            equal(maximum, 1_048_576)
        }
        other => Err(format!("region domain cap was enlarged: {other:?}").into()),
    }
}

#[test]
fn payloads_across_metadata_boundaries_have_exact_count_and_tail_lengths() -> TestResult {
    for target in [65_535, 65_536, 65_537, 131_072, 131_073, 1_048_575] {
        let dir = tempfile::tempdir()?;
        let store = Store::open_with(dir.path(), 8_388_608)?;
        let payload = RegionMembershipContractV1::new(sized_input(target)?)?;
        let reference = payload.reference()?;
        equal(
            store.register_region_contract(&reference, &payload)?,
            RegionContractRegistration::Registered(reference.clone()),
        )?;
        equal(store.snapshot().region_contract(&reference)?, payload)?;
        let key = keys::descriptor(&reference)?;
        let count = target.div_ceil(65_536);
        equal(store.meta.prefix(keys::chunk_prefix(&key)?).count(), count)?;
        let head = count
            .checked_sub(1)
            .ok_or("no chunk")?
            .checked_mul(65_536)
            .ok_or("head overflow")?;
        let last = store
            .meta
            .get(keys::chunk(
                &key,
                count.checked_sub(1).ok_or("last chunk")?,
            )?)?
            .ok_or("tail")?;
        equal(last.len(), target.checked_sub(head).ok_or("tail length")?)?;
    }
    Ok(())
}

#[test]
fn ordinary_metadata_encoder_retains_its_exact_64_kib_ceiling() -> TestResult {
    let at_cap = "x".repeat(65_534);
    equal(
        crate::records::codec::encode_canonical(&at_cap)?.len(),
        65_536,
    )?;
    match crate::records::codec::encode_canonical(&"x".repeat(65_535)) {
        Err(StoreError::RowTooLarge { bytes, max }) => equal((bytes, max), (65_537, 65_536)),
        other => Err(format!("ordinary envelope cap changed: {other:?}").into()),
    }
}

#[test]
fn zero_assertions_remain_a_retained_unknown_contract() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = RegionMembershipContractV1::new(input(1, vec![])?)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    equal(store.region_contract(&reference)?, payload)
}

#[test]
fn malformed_descriptor_counts_encoding_and_lengths_refuse_before_reconstruction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let original = descriptor_value(&store, &reference)?;
    for (field, value, problem) in [
        (
            "payload_bytes",
            serde_json::json!(0),
            RegionDescriptorProblem::EmptyPayload,
        ),
        (
            "chunk_bytes",
            serde_json::json!(65_537),
            RegionDescriptorProblem::ChunkSize,
        ),
        (
            "chunks",
            serde_json::json!(0),
            RegionDescriptorProblem::ChunkCount,
        ),
        (
            "chunks",
            serde_json::json!(17),
            RegionDescriptorProblem::ChunkCount,
        ),
        (
            "encoding",
            serde_json::json!("other_codec"),
            RegionDescriptorProblem::Encoding,
        ),
    ] {
        let mut descriptor = original.clone();
        descriptor
            .as_object_mut()
            .ok_or("descriptor")?
            .insert(field.to_string(), value);
        inject(
            &store,
            keys::descriptor(&reference)?.as_bytes(),
            &serde_json::to_vec(&descriptor)?,
        )?;
        match store.region_contract(&reference) {
            Err(StoreError::RegionContractRegistry(
                RegionContractRegistryError::InvalidDescriptor {
                    reference: held,
                    problem: observed,
                },
            )) => {
                equal(held, reference.clone())?;
                equal(observed, problem)?;
            }
            other => {
                return Err(format!("invalid descriptor reached reconstruction: {other:?}").into())
            }
        }
    }
    Ok(())
}

#[test]
fn oversized_descriptor_and_declared_payload_are_explicitly_bounded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let mut descriptor = descriptor_value(&store, &reference)?;
    descriptor
        .as_object_mut()
        .ok_or("descriptor")?
        .insert("payload_bytes".to_string(), serde_json::json!(1_048_577));
    for (bytes, expected) in [
        (vec![b' '; 65_537], ("descriptor", 65_537, 65_536)),
        (
            serde_json::to_vec(&descriptor)?,
            ("payload", 1_048_577, 1_048_576),
        ),
    ] {
        inject(&store, keys::descriptor(&reference)?.as_bytes(), &bytes)?;
        match store.region_contract(&reference) {
            Err(StoreError::RegionContractRegistry(RegionContractRegistryError::Oversized {
                what,
                bytes,
                max,
            })) => equal((what, bytes, max), expected)?,
            other => return Err(format!("oversized retained framing accepted: {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn direct_payload_decode_keeps_domain_one_mib_admission_boundary() -> TestResult {
    let reference = contract(7)?.reference()?;
    match wire::payload(&reference, &vec![b' '; 1_048_577]) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::InvalidPayload(
            census_domain::model::RegionMembershipError::PayloadTooLarge { maximum },
        ))) => equal(maximum, 1_048_576),
        other => Err(format!("raw domain admission cap was bypassed: {other:?}").into()),
    }
}
