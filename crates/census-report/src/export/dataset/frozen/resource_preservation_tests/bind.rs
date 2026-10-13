use super::super::freeze::{self, Staged};
use super::super::recover;
use super::super::root::{self, Retained};
use super::super::{objects, verify};
use super::fixture::{self, ResourceCase};
use super::{seeded_store, RetainedEvidence, Store, TestResult};
use census_store::InputState;

fn refused(store: &Store, staged: &Staged, case: ResourceCase) -> TestResult {
    let error = match recover::bind(store, staged) {
        Ok(_) => return Err("over-budget staged input was bound".into()),
        Err(error) => error,
    };
    fixture::check_error(&error, case)
}

fn preserved(case: ResourceCase) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("resource-preservation:bind:{case:?}");
    let directory = freeze::directory(&store, &job)?;
    std::fs::create_dir_all(objects::directory(&directory))?;
    let mut staged = freeze::stage(&store, &job, &directory)?;
    let current = freeze::root_of(&staged);
    let rewritten = fixture::rewrite(&store, &current, case)?;
    staged.manifest = rewritten.manifest;
    staged.manifest_digest = rewritten.digest;
    staged.inventory = rewritten.inventory;
    staged.bytes = rewritten.bytes;
    root::write_retained(
        &directory,
        &Retained {
            attempt: staged.attempt.clone(),
            manifest: staged.manifest_digest.clone(),
        },
    )?;
    let before = RetainedEvidence::capture(&store, &directory, &[&staged.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&store, &staged, case)?;
        before.unchanged(&store, &directory)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    before.unchanged(&reopened, &directory)?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&reopened, &staged, case)?;
        before.unchanged(&reopened, &directory)
    })?;
    let record = reopened
        .get_input_record(&staged.attempt)?
        .ok_or("staged input missing")?;
    check!(eq; record.state(), InputState::Freezing);
    check!(reopened.get_lease_record(&staged.attempt)?.is_some());
    fixture::check_cleanup_marker(&directory, case)
}

#[test]
fn resource_preservation_bind_keeps_the_record_root_and_lease_across_repeat_and_reopen(
) -> TestResult {
    [
        ResourceCase::InventoryChunk,
        ResourceCase::CursorLine,
        ResourceCase::CursorLineCleanup,
    ]
    .into_iter()
    .try_for_each(preserved)
}

#[test]
fn resource_preservation_real_inventory_refusal_is_typed_before_binding() -> TestResult {
    let seeded = seeded_store()?;
    let root = super::freeze_v2(&seeded.store, "resource-preservation:verification-error")?;
    let rewritten = fixture::rewrite(&seeded.store, &root, ResourceCase::InventoryChunk)?;
    let error = match verify::verify_root(
        &objects::directory(root.directory()),
        &rewritten.digest,
        &rewritten.manifest,
    ) {
        Ok(_) => return Err("over-budget inventory was verified".into()),
        Err(error) => error,
    };
    check!(matches!(
        &error,
        super::super::verification_error::VerificationError::Codec(
            super::super::codec::FrozenCodecError::RowTooLarge {
                what: "frozen chunk",
                ..
            }
        )
    ));
    fixture::check_error(&error.into(), ResourceCase::InventoryChunk)
}
