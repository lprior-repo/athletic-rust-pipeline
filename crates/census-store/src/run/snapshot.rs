use crate::{Store, Table};

use super::support::{
    equal, fixture_dir, inject, manifest, refused, seed, Refusal, TestResult, Witness,
};

#[test]
fn old_unbound_view_stays_unbound_after_binding_through_a_store_clone() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let clone = store.clone();
    let old = store.snapshot();
    let clone_old = clone.snapshot();
    let before = Witness::capture(&store)?;
    let identity = old.store_identity()?;
    equal(old.run_manifest()?, None)?;
    let bound = manifest(identity.as_str(), 1)?;
    clone.bind_run(&bound)?;
    let after = Witness::capture(&store)?;
    before.binding_only(&after, &serde_json::to_vec(&bound)?)?;
    for view in [&old, &clone_old] {
        view.ensure_origin(&clone)?;
        equal(view.run_manifest()?, None)?;
        equal(view.store_identity()?, identity.clone())?;
        equal(view.sequence(), before.sequence)?;
        equal(
            (view.evidence_generation(), view.derived_generation()),
            (before.evidence, before.derived),
        )?;
        equal(view.tables_digest(&Table::ALL)?, before.digest.clone())?;
    }
    equal(store.run_manifest()?, Some(bound.clone()))?;
    equal(store.snapshot().run_manifest()?, Some(bound.clone()))?;
    equal(clone.snapshot().run_manifest()?, Some(bound.clone()))?;
    equal(&Witness::capture(&store)?, &after)?;
    drop(old);
    drop(clone_old);
    drop(clone);
    drop(store);
    let reopened = Store::open_with(root.path(), 8_388_608)?;
    equal(reopened.run_manifest()?, Some(bound.clone()))?;
    equal(reopened.snapshot().run_manifest()?, Some(bound))?;
    equal(Witness::capture(&reopened)?, after)
}

#[test]
fn old_bound_view_never_falls_back_to_corrupted_live_metadata() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let bound = manifest(store.store_identity()?.as_str(), 1)?;
    store.bind_run(&bound)?;
    let old = store.snapshot();
    let sequence = old.sequence();
    inject(&store, b"{corrupt")?;
    let corrupted = Witness::capture(&store)?;
    equal(old.run_manifest()?, Some(bound))?;
    equal(old.sequence(), sequence)?;
    refused(store.snapshot().run_manifest(), Refusal::Json)?;
    refused(store.run_manifest(), Refusal::Json)?;
    equal(Witness::capture(&store)?, corrupted)
}

#[test]
fn an_exact_key_reader_ignores_adjacent_metadata() -> TestResult {
    let root = fixture_dir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    seed(&store)?;
    let mut batch = store.db.batch();
    batch.insert(&store.meta, "run-manifest-extra", b"{invalid".as_slice());
    batch.insert(&store.meta, "run-manifest\0neighbor", b"invalid".as_slice());
    batch.commit()?;
    let before = Witness::capture(&store)?;
    equal(store.run_manifest()?, None)?;
    equal(store.snapshot().run_manifest()?, None)?;
    equal(Witness::capture(&store)?, before)
}
