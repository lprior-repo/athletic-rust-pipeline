use crate::Store;
use census_domain::UsJurisdiction;

use super::support::{
    encoded_boundary, equal, fixture_dir, inject, manifest, oversized_write, raw_binding, refused,
    seed, Refusal, TestResult, Witness, LIMIT,
};

#[test]
fn run_record_byte_profile_is_exactly_65536_encoded_bytes() -> TestResult {
    equal(LIMIT, 65_536)
}

#[test]
fn exact_boundary_is_admitted_by_first_binding_and_both_readers() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let bound = encoded_boundary(LIMIT)?;
    let encoded = serde_json::to_vec(&bound)?;
    let before = Witness::capture(&store)?;
    store.bind_run(&bound)?;
    equal(raw_binding(&store)?, Some(encoded.clone()))?;
    equal(store.run_manifest()?, Some(bound.clone()))?;
    equal(store.snapshot().run_manifest()?, Some(bound.clone()))?;
    let after = Witness::capture(&store)?;
    before.binding_only(&after, &encoded)?;
    store.bind_run(&bound)?;
    equal(&Witness::capture(&store)?, &after)?;
    drop(store);
    let reopened = Store::open_with(root.path(), 8_388_608)?;
    equal(reopened.snapshot().run_manifest()?, Some(bound))?;
    equal(raw_binding(&reopened)?, Some(encoded))?;
    equal(Witness::capture(&reopened)?, after)
}

#[test]
fn oversized_first_bindings_create_no_key_and_consume_no_sequence() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let before = Witness::capture(&store)?;
    let one_over = LIMIT.checked_add(1).ok_or("fixture size overflow")?;
    let boundary = encoded_boundary(one_over)?;
    let mut huge_identity = manifest("", 1)?;
    huge_identity.store_identity = "x".repeat(LIMIT.checked_mul(2).ok_or("fixture size overflow")?);
    let mut excessive_scope = manifest("short", 1)?;
    excessive_scope.jurisdictions = vec![UsJurisdiction::Ohio; LIMIT];
    let mut escaped_identity = manifest("", 1)?;
    escaped_identity.store_identity = "\n".repeat(LIMIT / 2);
    for candidate in [boundary, huge_identity, excessive_scope, escaped_identity] {
        if serde_json::to_vec(&candidate)?.len() <= LIMIT {
            return Err("oversized synthetic fixture is within the byte ceiling".into());
        }
        oversized_write(store.bind_run(&candidate))?;
        equal(raw_binding(&store)?, None)?;
        equal(store.run_manifest()?, None)?;
        equal(store.snapshot().run_manifest()?, None)?;
        equal(&Witness::capture(&store)?, &before)?;
    }
    Ok(())
}

#[test]
fn retained_oversize_refuses_before_utf8_or_json_and_preserves_every_byte() -> TestResult {
    let one_over = LIMIT.checked_add(1).ok_or("fixture size overflow")?;
    let valid = serde_json::to_vec(&encoded_boundary(one_over)?)?;
    for raw in [valid, vec![0xff; one_over], vec![b'{'; one_over]] {
        let root = fixture_dir()?;
        let store = Store::open_with(root.path(), 8_388_608)?;
        seed(&store)?;
        inject(&store, &raw)?;
        let before = Witness::capture(&store)?;
        let candidate = manifest(store.store_identity()?.as_str(), 1)?;
        let pinned = store.snapshot();
        refused(pinned.run_manifest(), Refusal::Oversized(one_over))?;
        refused(store.run_manifest(), Refusal::Oversized(one_over))?;
        refused(
            store.bind_run(&candidate).map(|()| None),
            Refusal::Oversized(one_over),
        )?;
        equal(raw_binding(&store)?, Some(raw.clone()))?;
        equal(&Witness::capture(&store)?, &before)?;
        drop(pinned);
        drop(store);
        let reopened = Store::open_with(root.path(), 8_388_608)?;
        refused(reopened.run_manifest(), Refusal::Oversized(one_over))?;
        refused(
            reopened.snapshot().run_manifest(),
            Refusal::Oversized(one_over),
        )?;
        equal(raw_binding(&reopened)?, Some(raw))?;
        equal(Witness::capture(&reopened)?, before)?;
    }
    Ok(())
}

#[test]
fn malformed_metadata_refusals_match_and_never_become_absence_or_a_rebind() -> TestResult {
    let ordinary = manifest("retained-unconstrained-identity", 1)?;
    let valid = serde_json::to_value(&ordinary)?;
    let mut cases = vec![
        (vec![0xff, 0xfe], Refusal::Utf8),
        (b"{invalid".to_vec(), Refusal::Json),
    ];
    for key in ["store_identity", "run", "cohort", "jurisdictions"] {
        let mut missing = valid.clone();
        missing
            .as_object_mut()
            .ok_or("fixture is not an object")?
            .remove(key)
            .ok_or("fixture field absent")?;
        cases.push((serde_json::to_vec(&missing)?, Refusal::Json));
        let field = valid.get(key).ok_or("fixture field absent")?;
        let mut duplicate = serde_json::to_string(&valid)?;
        equal(duplicate.pop(), Some('}'))?;
        duplicate.push_str(&format!(",\"{key}\":{field}}}"));
        cases.push((duplicate.into_bytes(), Refusal::Json));
    }
    let encoded = serde_json::to_string(&ordinary)?;
    for raw in [
        encoded.replace("\"revision\":1", "\"revision\":0"),
        encoded.replace("\"season\":2026", "\"season\":1800"),
        encoded.replace("\"cohort\":2027", "\"cohort\":1800"),
        encoded.replace("\"OH\"", "\"ZZ\""),
        encoded.replace("\"run\":{", "\"run\":{\"extra\":1,"),
        encoded.replacen('{', "{\"extra\":1,", 1),
        "[]".to_owned(),
        "null".to_owned(),
    ] {
        cases.push((raw.into_bytes(), Refusal::Json));
    }
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    for (raw, expected) in cases {
        inject(&store, &raw)?;
        let before = Witness::capture(&store)?;
        let pinned = store.snapshot();
        refused(pinned.run_manifest(), expected)?;
        refused(store.run_manifest(), expected)?;
        refused(store.bind_run(&ordinary).map(|()| None), expected)?;
        equal(raw_binding(&store)?, Some(raw))?;
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}
