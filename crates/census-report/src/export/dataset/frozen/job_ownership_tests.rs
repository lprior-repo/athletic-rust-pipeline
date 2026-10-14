use super::fixtures::{seeded_store, TestResult};
use super::root::{self, Retained};
use super::version_records::{install_record, RetainedState};
use super::version_snapshot::RetainedEvidence;
use super::{cancel_freeze_v2, freeze_v2, objects, verify, FrozenRoot};
use crate::report::ReportError;
use census_domain::model::serialized_digest;
use census_store::Store;

const FIRST_JOB: &str = "job-ownership:first";
const SECOND_JOB: &str = "job-ownership:second";
const FOREIGN_JOB: &str = "a retained frozen root belongs to a different logical job";
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

fn refused(store: &Store, operation: Operation) -> TestResult {
    let error = match operation {
        Operation::Resume => match freeze_v2(store, FIRST_JOB) {
            Ok(_) => return Err("a foreign-job root was resumed or replaced".into()),
            Err(error) => error,
        },
        Operation::Cancel => match cancel_freeze_v2(store, FIRST_JOB) {
            Ok(_) => return Err("a foreign-job root was cancelled or swept".into()),
            Err(error) => error,
        },
    };
    check!(matches!(error, ReportError::Invariant { detail } if detail == FOREIGN_JOB));
    Ok(())
}

fn install_foreign(
    store: &Store,
    first: &FrozenRoot,
    second: &FrozenRoot,
    state: RetainedState,
) -> TestResult<Retained> {
    check!(eq; first.manifest().lineage.export_job.as_deref(), Some(FIRST_JOB));
    check!(eq; second.manifest().lineage.export_job.as_deref(), Some(SECOND_JOB));
    check!(ne; first.directory(), second.directory());
    let objects_dir = objects::directory(first.directory());
    for entry in std::fs::read_dir(objects::directory(second.directory()))? {
        let entry = entry?;
        std::fs::copy(entry.path(), objects_dir.join(entry.file_name()))?;
    }
    let inventory = verify::verify_root(&objects_dir, second.manifest_digest(), second.manifest())?;
    let retained = Retained {
        attempt: serialized_digest(&(
            "foreign-job-ownership-fixture",
            second.manifest_digest(),
            format!("{state:?}"),
        ))?,
        manifest: second.manifest_digest().to_string(),
    };
    if let Some(record) =
        install_record(store, &retained.attempt, second.bytes(), &inventory, state)?
    {
        verify::check_record(second.manifest(), &record, &inventory)?;
    }
    root::write_retained(first.directory(), &retained)?;
    Ok(retained)
}

fn preserved(state: RetainedState, operation: Operation) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let first = freeze_v2(&store, FIRST_JOB)?;
    let second = freeze_v2(&store, SECOND_JOB)?;
    let retained = install_foreign(&store, &first, &second, state)?;
    let attempts = [first.attempt(), second.attempt(), retained.attempt.as_str()];
    let first_before = RetainedEvidence::capture(&store, first.directory(), &attempts)?;
    let second_before = RetainedEvidence::capture(&store, second.directory(), &attempts)?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&store, operation)?;
        first_before.unchanged(&store, first.directory())?;
        second_before.unchanged(&store, second.directory())
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    first_before.unchanged(&reopened, first.directory())?;
    second_before.unchanged(&reopened, second.directory())?;
    (0..2).try_for_each(|_| -> TestResult {
        refused(&reopened, operation)?;
        first_before.unchanged(&reopened, first.directory())?;
        second_before.unchanged(&reopened, second.directory())
    })?;
    check!(eq; root::read_retained(first.directory())?, Some(retained));
    Ok(())
}

#[test]
fn freeze_job_ownership_requested_refuses_foreign_root_without_mutation() -> TestResult {
    preserved(RetainedState::Requested, Operation::Resume)
}

#[test]
fn freeze_job_ownership_freezing_refuses_foreign_root_without_mutation() -> TestResult {
    preserved(RetainedState::Freezing, Operation::Resume)
}

#[test]
fn freeze_job_ownership_frozen_refuses_foreign_root_without_mutation() -> TestResult {
    preserved(RetainedState::Frozen, Operation::Resume)
}

#[test]
fn freeze_job_ownership_missing_and_abandoned_preserve_foreign_root() -> TestResult {
    [RetainedState::Missing, RetainedState::Abandoned]
        .into_iter()
        .try_for_each(|state| preserved(state, Operation::Resume))
}

#[test]
fn cancellation_job_ownership_preserves_foreign_roots_in_every_state_after_reopen() -> TestResult {
    STATES
        .into_iter()
        .try_for_each(|state| preserved(state, Operation::Cancel))
}

const MIXED_ROOT: &str = "a retained frozen root does not match its durable input record";
const MIXED_STATES: [RetainedState; 4] = [
    RetainedState::Requested,
    RetainedState::Freezing,
    RetainedState::Frozen,
    RetainedState::Abandoned,
];

fn mixed_refused(store: &Store, operation: Operation) -> TestResult {
    let error = match operation {
        Operation::Resume => match freeze_v2(store, FIRST_JOB) {
            Ok(_) => return Err("a mixed root was resumed or replaced".into()),
            Err(error) => error,
        },
        Operation::Cancel => match cancel_freeze_v2(store, FIRST_JOB) {
            Ok(_) => return Err("a mixed root was cancelled or accepted".into()),
            Err(error) => error,
        },
    };
    check!(matches!(error, ReportError::Invariant { detail } if detail == MIXED_ROOT));
    Ok(())
}

fn install_mixed(
    store: &Store,
    first: &FrozenRoot,
    second: &FrozenRoot,
    state: RetainedState,
) -> TestResult<Retained> {
    let inventory = verify::verify_root(
        &objects::directory(second.directory()),
        second.manifest_digest(),
        second.manifest(),
    )?;
    let second_retained = Retained {
        attempt: serialized_digest(&(
            "mixed-job-ownership-fixture",
            second.manifest_digest(),
            format!("{state:?}"),
        ))?,
        manifest: second.manifest_digest().to_string(),
    };
    let record = install_record(
        store,
        &second_retained.attempt,
        second.bytes(),
        &inventory,
        state,
    )?
    .ok_or("a mixed-root fixture requires an existing second-job record")?;
    verify::check_record(second.manifest(), &record, &inventory)?;
    check!(eq; record.chunks().first().map(String::as_str), Some(second.manifest_digest()));
    check!(ne; first.manifest_digest(), second.manifest_digest());
    root::write_retained(second.directory(), &second_retained)?;
    let mixed = Retained {
        attempt: second_retained.attempt,
        manifest: first.manifest_digest().to_string(),
    };
    root::write_retained(first.directory(), &mixed)?;
    Ok(mixed)
}

fn mixed_preserved(state: RetainedState, operation: Operation) -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let first = freeze_v2(&store, FIRST_JOB)?;
    let second = freeze_v2(&store, SECOND_JOB)?;
    let retained = install_mixed(&store, &first, &second, state)?;
    let attempts = [first.attempt(), second.attempt(), retained.attempt.as_str()];
    let first_before = RetainedEvidence::capture(&store, first.directory(), &attempts)?;
    let second_before = RetainedEvidence::capture(&store, second.directory(), &attempts)?;
    (0..2).try_for_each(|_| -> TestResult {
        mixed_refused(&store, operation)?;
        first_before.unchanged(&store, first.directory())?;
        second_before.unchanged(&store, second.directory())
    })?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    first_before.unchanged(&reopened, first.directory())?;
    second_before.unchanged(&reopened, second.directory())?;
    (0..2).try_for_each(|_| -> TestResult {
        mixed_refused(&reopened, operation)?;
        first_before.unchanged(&reopened, first.directory())?;
        second_before.unchanged(&reopened, second.directory())
    })?;
    check!(eq; root::read_retained(first.directory())?, Some(retained));
    Ok(())
}

#[test]
fn freeze_job_ownership_mixed_pointer_preserves_both_jobs_after_reopen() -> TestResult {
    MIXED_STATES
        .into_iter()
        .try_for_each(|state| mixed_preserved(state, Operation::Resume))
}

#[test]
fn cancellation_job_ownership_mixed_pointer_preserves_both_jobs_after_reopen() -> TestResult {
    MIXED_STATES
        .into_iter()
        .try_for_each(|state| mixed_preserved(state, Operation::Cancel))
}
