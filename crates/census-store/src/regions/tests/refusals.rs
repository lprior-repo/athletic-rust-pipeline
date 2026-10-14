use super::*;

#[test]
fn missing_revision_and_digest_never_fall_back_or_mutate_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let before = Witness::capture(&store)?;
    match store.region_contract(&reference) {
        Err(RegionRegistryError::Missing { reference: found }) => equal(found, reference.clone())?,
        other => return Err(format!("missing contract={other:?}").into()),
    }
    equal(Witness::capture(&store)?, before)?;
    store.register_region_contract(&payload)?;
    let before = Witness::capture(&store)?;
    let revision = ContractRefV1::new(1, 8, reference.digest())?;
    let digest = ContractRefV1::new(1, 7, &"0".repeat(64))?;
    for requested in [revision, digest] {
        for result in [
            store.region_contract(&requested),
            store.snapshot().region_contract(&requested),
        ] {
            match result {
                Err(RegionRegistryError::Missing { reference: found }) => {
                    equal(found, requested.clone())?;
                }
                other => return Err(format!("tampered lookup={other:?}").into()),
            }
        }
    }
    equal(store.region_contract(&reference)?, payload)?;
    equal(Witness::capture(&store)?, before)
}

#[test]
fn unknown_versions_refuse_before_full_payload_deserialization() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    for (bytes, expected_what) in [
        (
            br#"{"version":2,"payload":false}"#.to_vec(),
            "region artifact envelope",
        ),
        (
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "reference": reference,
                "payload": {"version": 2, "not_a_current_contract": true}
            }))?,
            "region membership contract",
        ),
    ] {
        inject(&store, &reference, &bytes)?;
        for error in unchanged_refusals(&store, &payload)? {
            match error {
                RegionRegistryError::UnsupportedVersion { what, found } => {
                    equal(what, expected_what)?;
                    equal(found, 2)?;
                }
                other => return Err(format!("unknown version={other:?}").into()),
            }
        }
    }
    Ok(())
}

#[test]
fn malformed_and_foreign_records_are_not_repaired_or_replaced() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    for bytes in [b"".as_slice(), b"{", b"null", b"[]", b"{}", b"corrupted"] {
        inject(&store, &reference, bytes)?;
        for error in unchanged_refusals(&store, &payload)? {
            match error {
                RegionRegistryError::Malformed(_) => {}
                other => return Err(format!("malformed record={other:?}").into()),
            }
        }
    }
    Ok(())
}

#[test]
fn foreign_fields_and_invalid_current_payloads_refuse_without_writes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    for path in [
        "",
        "/reference",
        "/payload",
        "/payload/assertions/0/provenance",
    ] {
        let mut value = envelope_value(&payload)?;
        let object = value
            .pointer_mut(path)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("missing fixture object")?;
        equal(
            object.insert("foreign".to_string(), serde_json::json!(true)),
            None,
        )?;
        inject(&store, &reference, &serde_json::to_vec(&value)?)?;
        for error in unchanged_refusals(&store, &payload)? {
            match error {
                RegionRegistryError::Malformed(_) => {}
                other => return Err(format!("foreign field at {path}={other:?}").into()),
            }
        }
    }
    for (path, invalid) in [
        ("/reference/version", serde_json::json!(2)),
        ("/payload/revision", serde_json::json!(0)),
        ("/payload/jurisdictions", serde_json::json!([])),
        ("/payload/jurisdictions", serde_json::json!(vec!["MI"; 52])),
        (
            "/payload/assertions/0/provenance/statement",
            serde_json::json!("x".repeat(513)),
        ),
    ] {
        let mut value = envelope_value(&payload)?;
        replace(&mut value, path, invalid)?;
        inject(&store, &reference, &serde_json::to_vec(&value)?)?;
        for error in unchanged_refusals(&store, &payload)? {
            match error {
                RegionRegistryError::Malformed(_) => {}
                other => return Err(format!("invalid current payload at {path}={other:?}").into()),
            }
        }
    }
    Ok(())
}

#[test]
fn envelope_reference_revision_and_digest_tampering_cannot_be_overwritten() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let revision = ContractRefV1::new(1, 8, reference.digest())?;
    let digest = ContractRefV1::new(1, 7, &"0".repeat(64))?;
    for claimed in [revision, digest] {
        let bytes = serde_json::to_vec(&Envelope {
            version: 1,
            reference: &claimed,
            payload: &payload,
        })?;
        inject(&store, &reference, &bytes)?;
        for error in unchanged_refusals(&store, &payload)? {
            match error {
                RegionRegistryError::ReferenceMismatch => {}
                other => return Err(format!("claimed reference tampering={other:?}").into()),
            }
        }
    }
    Ok(())
}

#[test]
fn recomputed_payload_revision_and_digest_must_match_requested_reference() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    for (path, tampered) in [
        ("/payload/revision", serde_json::json!(8)),
        (
            "/payload/region",
            serde_json::json!("synthetic:different-region"),
        ),
        (
            "/payload/assertions/0/provenance/capture_sha256",
            serde_json::json!("c".repeat(64)),
        ),
    ] {
        let mut value = envelope_value(&payload)?;
        replace(&mut value, path, tampered)?;
        inject(&store, &reference, &serde_json::to_vec(&value)?)?;
        for error in unchanged_refusals(&store, &payload)? {
            match error {
                RegionRegistryError::ReferenceMismatch => {}
                other => return Err(format!("payload tampering at {path}={other:?}").into()),
            }
        }
    }
    Ok(())
}

#[test]
fn admitted_set_normalization_reads_without_rewriting_original_record() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), CACHE_BYTES)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    let mut value = envelope_value(&payload)?;
    let assertions = value
        .pointer_mut("/payload/assertions")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("fixture assertions")?;
    assertions.reverse();
    let duplicate = assertions.first().ok_or("fixture assertion")?.clone();
    assertions.push(duplicate);
    replace(
        &mut value,
        "/payload/jurisdictions",
        serde_json::json!(["MI", "MI"]),
    )?;
    inject(&store, &reference, &serde_json::to_vec(&value)?)?;
    let before = Witness::capture(&store)?;
    equal(store.region_contract(&reference)?, payload.clone())?;
    equal(
        store.snapshot().region_contract(&reference)?,
        payload.clone(),
    )?;
    equal(
        store.register_region_contract(&payload)?,
        RegionRegistration::Repeated(reference),
    )?;
    equal(Witness::capture(&store)?, before)
}
