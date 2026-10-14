use super::fixtures::{seeded_store, TestResult};
use super::root;
use super::version_records::RetainedState;
use super::version_snapshot::RetainedEvidence;
use super::{cancel_freeze_v2, freeze_v2, version_records};
use census_store::Store;
use fixture::ResourceCase;

#[path = "resource_preservation_tests/bind.rs"]
mod bind;
#[path = "resource_preservation_tests/fixture.rs"]
mod fixture;

const STATES: [RetainedState; 5] = [
    RetainedState::Missing,
    RetainedState::Requested,
    RetainedState::Freezing,
    RetainedState::Frozen,
    RetainedState::Abandoned,
];

#[derive(Debug, Clone, Copy)]
enum Operation {
    Resume,
    Cancel,
}

fn refused(store: &Store, job: &str, case: ResourceCase, operation: Operation) -> TestResult {
    let error = match operation {
        Operation::Resume => match freeze_v2(store, job) {
            Ok(_) => return Err("resource-refused input was resumed or replaced".into()),
            Err(error) => error,
        },
        Operation::Cancel => match cancel_freeze_v2(store, job) {
            Ok(_) => return Err("resource-refused input was cancelled or swept".into()),
            Err(error) => error,
        },
    };
    fixture::check_error(&error, case)
}

fn preserved(case: ResourceCase, state: RetainedState, operation: Operation) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let job = format!("resource-preservation:{case:?}:{state:?}:{operation:?}");
    let current = freeze_v2(&store, &job)?;
    let directory = current.directory().to_path_buf();
    let retained = fixture::install(&store, &current, case, state)?;
    let before =
        RetainedEvidence::capture(&store, &directory, &[current.attempt(), &retained.attempt])?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&store, &job, case, operation)?;
        before.unchanged(&store, &directory)?;
        fixture::check_cleanup_marker(&directory, case)
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    before.unchanged(&reopened, &directory)?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&reopened, &job, case, operation)?;
        before.unchanged(&reopened, &directory)?;
        fixture::check_cleanup_marker(&directory, case)
    })?;
    check!(eq; root::read_retained(&directory)?, Some(retained));
    Ok(())
}

fn matrix(operation: Operation) -> TestResult {
    ResourceCase::ALL.into_iter().try_for_each(|case| {
        STATES
            .into_iter()
            .try_for_each(|state| preserved(case, state, operation))
    })
}

#[test]
fn resource_preservation_resume_retains_every_state_across_repeat_and_reopen() -> TestResult {
    matrix(Operation::Resume)
}

#[test]
fn resource_preservation_cancel_retains_every_state_across_repeat_and_reopen() -> TestResult {
    matrix(Operation::Cancel)
}

#[test]
fn resource_preservation_frozen_reuse_keeps_primary_cleanup_resource_errors() -> TestResult {
    preserved(
        ResourceCase::CursorLineCleanup,
        RetainedState::Frozen,
        Operation::Resume,
    )
}

#[test]
fn resource_preservation_recovery_keeps_inventory_resource_errors() -> TestResult {
    [RetainedState::Requested, RetainedState::Freezing]
        .into_iter()
        .try_for_each(|state| preserved(ResourceCase::InventoryChunk, state, Operation::Resume))
}
