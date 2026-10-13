use crate::{Store, StoreError};

use super::process::bounded_case;
use super::support::{
    canonical, envelope, equal, fixture_dir, inject, refused, seed, Refusal, TestResult, Witness,
    LOWER, PHASE, UPPER,
};

#[test]
fn snapshots_before_initialization_remain_missing_after_live_creation() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    seed(&store)?;
    let missing = store.snapshot();
    let before = Witness::capture(&store)?;
    refused(missing.store_identity(), Refusal::Missing)?;
    equal(&Witness::capture(&store)?, &before)?;
    let identity = store.store_identity()?;
    let after = Witness::capture(&store)?;
    refused(missing.store_identity(), Refusal::Missing)?;
    equal(missing.sequence(), before.sequence)?;
    equal(store.snapshot().store_identity()?, identity)?;
    equal(Witness::capture(&store)?, after)
}

#[test]
fn retained_snapshot_does_not_fall_back_to_replaced_live_lineage() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    inject(&store, &[canonical(&envelope(UPPER))])?;
    let retained = store.snapshot();
    equal(retained.store_identity()?.as_str(), UPPER)?;
    inject(&store, &[canonical(&envelope("bad"))])?;
    let replaced = Witness::capture(&store)?;
    equal(retained.store_identity()?.as_str(), UPPER)?;
    refused(store.snapshot().store_identity(), Refusal::InvalidIdentity)?;
    refused(store.store_identity(), Refusal::InvalidIdentity)?;
    equal(Witness::capture(&store)?, replaced)?;
    inject(
        &store,
        &[(Store::journal_key(PHASE, "extra"), envelope(LOWER))],
    )?;
    let multiple = Witness::capture(&store)?;
    equal(retained.store_identity()?.as_str(), UPPER)?;
    refused(store.snapshot().store_identity(), Refusal::Multiple)?;
    refused(store.store_identity(), Refusal::Multiple)?;
    equal(Witness::capture(&store)?, multiple)
}

#[test]
fn origin_checks_accept_clones_and_old_views_without_generation_validation() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    inject(&store, &[canonical(&envelope(UPPER))])?;
    let old = store.snapshot();
    let clone = store.clone();
    old.ensure_origin(&clone)?;
    equal(old.evidence_generation(), 0)?;
    seed(&store)?;
    equal(store.evidence_generation(), 1)?;
    equal(old.evidence_generation(), 0)?;
    old.ensure_origin(&store)?;
    old.ensure_origin(&clone)?;
    equal(old.store_identity()?.as_str(), UPPER)
}

#[test]
fn same_logical_identity_does_not_authorize_a_foreign_snapshot_origin() -> TestResult {
    let dir = fixture_dir()?;
    let a = Store::open_with(dir.path().join("a"), 8_388_608)?;
    let b = Store::open_with(dir.path().join("b"), 8_388_608)?;
    inject(&a, &[canonical(&envelope(UPPER))])?;
    inject(&b, &[canonical(&envelope(UPPER))])?;
    equal(a.store_identity()?, b.store_identity()?)?;
    let foreign = b.snapshot();
    match foreign.ensure_origin(&a) {
        Err(StoreError::RevisionOriginMismatch { operation }) => {
            if operation.is_empty() {
                return Err("the foreign-origin refusal lost its operation context".into());
            }
        }
        observed => return Err(format!("foreign origin unexpectedly returned {observed:?}").into()),
    }
    foreign.ensure_origin(&b)?;
    foreign.ensure_origin(&b.clone()).map_err(Into::into)
}

#[test]
fn initialized_fenced_identity_reads_complete_on_the_fence_owner_thread() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::initialized_fenced_identity_reads_complete_on_the_fence_owner_thread"
        ),
        || {
            let dir = fixture_dir()?;
            let store = Store::open_with(dir.path(), 8_388_608)?;
            seed(&store)?;
            let identity = store.store_identity()?;
            let before = Witness::capture(&store)?;
            {
                let fence = store.fenced_snapshot();
                fence.view().ensure_origin(&store.clone())?;
                equal(fence.view().store_identity()?, identity)?;
                equal(fence.view().sequence(), before.sequence)?;
            }
            equal(Witness::capture(&store)?, before)
        },
    )
}

#[test]
fn missing_fenced_identity_reads_refuse_on_the_fence_owner_thread_without_writing() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::missing_fenced_identity_reads_refuse_on_the_fence_owner_thread_without_writing"
        ),
        || {
            let dir = fixture_dir()?;
            let store = Store::open_with(dir.path(), 8_388_608)?;
            seed(&store)?;
            let before = Witness::capture(&store)?;
            {
                let fence = store.fenced_snapshot();
                fence.view().ensure_origin(&store)?;
                refused(fence.view().store_identity(), Refusal::Missing)?;
            }
            equal(Witness::capture(&store)?, before)
        },
    )
}
