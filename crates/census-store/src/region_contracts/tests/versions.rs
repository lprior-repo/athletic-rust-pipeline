use super::*;

#[test]
fn future_descriptor_version_refuses_before_reference_shape_or_missing_payload() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let reference = contract(7)?.reference()?;
    inject(
        &store,
        keys::descriptor(&reference)?.as_bytes(),
        br#"{"version":2,"future_shape":true}"#,
    )?;
    let before = Witness::capture(&store)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            RegionContractRegistryError::UnsupportedVersion { what, found },
        )) => equal((what, found), ("region descriptor", 2))?,
        other => return Err(format!("future descriptor was interpreted as v1: {other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}

#[test]
fn future_reference_version_refuses_before_remaining_descriptor_fields() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let reference = contract(7)?.reference()?;
    inject(
        &store,
        keys::descriptor(&reference)?.as_bytes(),
        br#"{"version":1,"reference":{"version":2}}"#,
    )?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            RegionContractRegistryError::UnsupportedVersion { what, found },
        )) => equal((what, found), ("region descriptor reference", 2)),
        other => Err(format!("future reference was interpreted as v1: {other:?}").into()),
    }
}

#[test]
fn future_payload_version_refuses_without_assuming_v1_semantic_shape() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let reference = contract(7)?.reference()?;
    inject_payload(&store, &reference, br#"{"version":2,"future_shape":true}"#)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(
            RegionContractRegistryError::UnsupportedVersion { what, found },
        )) => equal((what, found), ("region payload", 2)),
        other => Err(format!("future payload was interpreted as v1: {other:?}").into()),
    }
}

#[test]
fn malformed_json_unknown_fields_and_invalid_payload_have_explicit_refusals() -> TestResult {
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
        .insert("extra".to_string(), serde_json::json!(1));
    for bytes in [b"{broken".to_vec(), serde_json::to_vec(&descriptor)?] {
        inject(&store, keys::descriptor(&reference)?.as_bytes(), &bytes)?;
        match store.region_contract(&reference) {
            Err(StoreError::RegionContractRegistry(RegionContractRegistryError::Malformed {
                reference: held,
                what,
                source,
            })) => {
                equal(held, reference.clone())?;
                equal(what, "descriptor")?;
                equal(source.is_syntax() || source.is_data(), true)?;
            }
            other => return Err(format!("malformed descriptor accepted: {other:?}").into()),
        }
    }
    let mut input = serde_json::to_value(payload.input())?;
    input
        .as_object_mut()
        .ok_or("input")?
        .insert("revision".to_string(), serde_json::json!(0));
    inject_payload(&store, &reference, &serde_json::to_vec(&input)?)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::InvalidPayload(
            census_domain::model::RegionMembershipError::ZeroRevision,
        ))) => Ok(()),
        other => Err(format!("invalid semantic payload accepted: {other:?}").into()),
    }
}

#[test]
fn unknown_payload_fields_are_refused_by_the_domain_boundary() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let mut input = serde_json::to_value(payload.input())?;
    input
        .as_object_mut()
        .ok_or("payload")?
        .insert("extra".to_string(), serde_json::json!(true));
    inject_payload(&store, &reference, &serde_json::to_vec(&input)?)?;
    let before = Witness::capture(&store)?;
    match store.region_contract(&reference) {
        Err(StoreError::RegionContractRegistry(RegionContractRegistryError::Malformed {
            reference: held,
            what,
            source,
        })) => {
            equal(held, reference)?;
            equal(what, "payload")?;
            equal(source.is_data(), true)?;
        }
        other => return Err(format!("unknown domain payload field accepted: {other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)
}
