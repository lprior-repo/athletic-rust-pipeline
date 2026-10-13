use crate::Store;
use census_domain::model::RunManifest;

use super::support::{
    equal, fixture_dir, inject, manifest, raw_binding, seed, TestResult, Witness, HISTORICAL,
};

#[test]
fn first_binding_preserves_existing_json_field_and_jurisdiction_order() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let bound = manifest(
        "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789",
        1,
    )?;
    let golden = br#"{"store_identity":"ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789","run":{"season":2026,"revision":1},"cohort":2027,"jurisdictions":["OH","NY","OH"]}"#;
    equal(serde_json::to_vec(&bound)?.as_slice(), golden.as_slice())?;
    let before = Witness::capture(&store)?;
    store.bind_run(&bound)?;
    let after = Witness::capture(&store)?;
    before.binding_only(&after, golden)?;
    equal(
        raw_binding(&store)?.ok_or("binding missing")?.as_slice(),
        golden.as_slice(),
    )?;
    equal(store.run_manifest()?, Some(bound.clone()))?;
    equal(store.snapshot().run_manifest()?, Some(bound))?;
    equal(Witness::capture(&store)?, after)
}

#[test]
fn retained_reordered_json_and_uppercase_identity_survive_read_rebind_and_reopen() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    inject(&store, HISTORICAL)?;
    let expected: RunManifest = serde_json::from_slice(HISTORICAL)?;
    let before = Witness::capture(&store)?;
    for _ in 0..3 {
        equal(store.run_manifest()?, Some(expected.clone()))?;
        equal(store.snapshot().run_manifest()?, Some(expected.clone()))?;
        store.bind_run(&expected)?;
        equal(
            raw_binding(&store)?.ok_or("binding missing")?.as_slice(),
            HISTORICAL,
        )?;
        equal(&Witness::capture(&store)?, &before)?;
    }
    drop(store);
    let reopened = Store::open_with(root.path(), 8_388_608)?;
    equal(reopened.run_manifest()?, Some(expected.clone()))?;
    equal(reopened.snapshot().run_manifest()?, Some(expected))?;
    equal(
        raw_binding(&reopened)?.ok_or("binding missing")?.as_slice(),
        HISTORICAL,
    )?;
    equal(Witness::capture(&reopened)?, before)
}

#[test]
fn admitted_legacy_string_identity_is_not_replaced_by_a_new_identity_parser() -> TestResult {
    for identity in ["", "legacy-opaque-identity", "NOT-A-SHA256"] {
        let root = fixture_dir()?;
        let store = Store::open_with(root.path(), 8_388_608)?;
        seed(&store)?;
        let retained = manifest(identity, 1)?;
        let bytes = serde_json::to_vec(&retained)?;
        inject(&store, &bytes)?;
        let before = Witness::capture(&store)?;
        equal(store.run_manifest()?, Some(retained.clone()))?;
        equal(store.snapshot().run_manifest()?, Some(retained.clone()))?;
        store.bind_run(&retained)?;
        equal(raw_binding(&store)?, Some(bytes))?;
        equal(Witness::capture(&store)?, before)?;
    }
    Ok(())
}
