use super::*;

fn refused(bundle: &wire::Bundle) -> TestResult {
    let bytes = serde_json::to_vec(bundle)?;
    check!(replay_frozen_public_identity_v2(&bytes, &super::super::super::digest(&bytes)).is_err());
    Ok(())
}

fn refused_as(bundle: &wire::Bundle, expected: &'static str) -> TestResult {
    let bytes = serde_json::to_vec(bundle)?;
    check!(
        matches!(replay_frozen_public_identity_v2(&bytes, &super::super::super::digest(&bytes)),
        Err(crate::StoreError::PublicIdentity(crate::PublicIdentityError::Corrupt(reason))) if reason == expected)
    );
    Ok(())
}

#[test]
fn missing_duplicate_and_unbound_object_inventory_refuse() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let removed = bundle.objects.pop().ok_or("object")?;
    refused(&bundle)?;
    bundle.objects.push(removed);
    let duplicate = serde_json::from_slice(&serde_json::to_vec(
        bundle.objects.first().ok_or("object")?,
    )?)?;
    bundle.objects.push(duplicate);
    refused_as(&bundle, "duplicate frozen graph object")?;
    let (_empty_directory, empty, _writer) = fixture()?;
    let empty_frozen = empty.snapshot().freeze_public_identity_v2()?;
    let mut empty_bundle: wire::Bundle = serde_json::from_slice(empty_frozen.bytes())?;
    empty_bundle
        .objects
        .push(bundle.objects.pop().ok_or("object")?);
    refused_as(&empty_bundle, "frozen graph reachable inventory")
}

#[test]
fn missing_altered_extra_and_reordered_physical_rows_refuse() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    let object = bundle.objects.first_mut().ok_or("object")?;
    let removed = object.rows.pop().ok_or("row")?;
    refused(&bundle)?;
    bundle
        .objects
        .first_mut()
        .ok_or("object")?
        .rows
        .push(removed);
    let object = bundle.objects.first_mut().ok_or("object")?;
    let row = object.rows.first_mut().ok_or("row")?;
    let old = std::mem::replace(&mut row.value_base64, "eA==".into());
    refused(&bundle)?;
    bundle
        .objects
        .first_mut()
        .ok_or("object")?
        .rows
        .first_mut()
        .ok_or("row")?
        .value_base64 = old;
    let object = bundle.objects.first_mut().ok_or("object")?;
    let duplicate =
        serde_json::from_slice(&serde_json::to_vec(object.rows.first().ok_or("row")?)?)?;
    object.rows.push(duplicate);
    refused_as(&bundle, "frozen graph physical row owner or order")?;
    let object = bundle.objects.first_mut().ok_or("object")?;
    object.rows.pop();
    object.rows.reverse();
    refused_as(&bundle, "frozen graph physical row owner or order")
}

#[test]
fn wrong_bundle_family_digest_and_historical_witness_refuse() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    check!(replay_frozen_public_identity_v2(frozen.bytes(), &"0".repeat(64)).is_err());
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    bundle.version = 1;
    refused_as(&bundle, "frozen graph bundle version")?;
    bundle.version = 2;
    let operation = bundle.operations.first_mut().ok_or("operation")?;
    let mut witness: serde_json::Value = serde_json::from_str(&operation.witness)?;
    *witness.get_mut("version").ok_or("witness version")? = serde_json::json!(2);
    operation.witness = serde_json::to_string(&witness)?;
    refused_as(&bundle, "graph witness schema binding")
}
