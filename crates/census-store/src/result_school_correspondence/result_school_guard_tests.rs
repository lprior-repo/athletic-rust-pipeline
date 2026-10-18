use super::*;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Store, StoreError, StoreResult};

const PHASE: &str = "result_school_correspondence";
const ENROLLMENT: &str = "result_school_correspondence/v1/enrollment";
const RECORD: &str = "result_school_correspondence/v1/record/00000000000000000001";
const OPERATION: &str = "result_school_correspondence_v1:protected-route";

fn fixture() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal.insert(
        Store::journal_key(PHASE, ENROLLMENT),
        b"retained enrollment bytes",
    )?;
    store
        .receipts
        .insert(OPERATION.as_bytes(), b"retained receipt bytes")?;
    store.entities.insert(b"preserved entity", b"original")?;
    store.meta.insert(b"preserved metadata", b"original")?;
    Ok((directory, store))
}

fn reserved<T>(result: StoreResult<T>) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::ResultSchoolCorrespondence(
            ResultSchoolCorrespondenceStoreError::ReservedWrite
        ))
    ));
    Ok(())
}

fn staged(store: &Store) -> TestResult<crate::StoreBatch<'_>> {
    let mut batch = store.write_batch();
    batch.journal_done("ordinary", "must-not-commit", &true)?;
    Ok(batch)
}

#[test]
fn direct_and_locked_journal_paths_refuse_reserved_phase() -> TestResult {
    let (_directory, store) = fixture()?;
    let before = retained_state(&store)?;
    for phase in [PHASE, "result_school_correspondence/v1/record"] {
        reserved(store.journal_done(phase, RECORD, &false))?;
        check!(eq; retained_state(&store)?, before);
        {
            let _guard = store.lock_appends();
            reserved(store.journal_done_locked(phase, RECORD, &false))?;
        }
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn batch_and_derived_journal_routes_cannot_overwrite_protected_enrollment() -> TestResult {
    let (_directory, store) = fixture()?;
    let before = retained_state(&store)?;
    for phase in [PHASE, "result_school_correspondence/v1/enrollment"] {
        let mut batch = staged(&store)?;
        reserved(batch.journal_done(phase, ENROLLMENT, &false))?;
        drop(batch);
        check!(eq; retained_state(&store)?, before);
    }
    let mut stage = store.stage_derived()?;
    let after_reservation = retained_state(&store)?;
    reserved(stage.journal_done_once(PHASE, ENROLLMENT, &false))?;
    check!(eq; retained_state(&store)?, after_reservation);
    check!(matches!(
        stage.journal_done_once("result_school_correspondence\0foreign", ENROLLMENT, &false),
        Err(StoreError::Refused { detail })
            if detail == "publication journal requires nonempty unambiguous phase and key"
    ));
    check!(eq; retained_state(&store)?, after_reservation);
    reserved(stage.publish(OPERATION, "different"))?;
    check!(eq; retained_state(&store)?, after_reservation);
    Ok(())
}

#[test]
fn every_generic_receipt_commit_route_refuses_before_staged_effects() -> TestResult {
    let (_directory, store) = fixture()?;
    let before = retained_state(&store)?;
    reserved(staged(&store)?.commit_once(OPERATION, "different"))?;
    check!(eq; retained_state(&store)?, before);
    reserved(staged(&store)?.commit_once_at_evidence_generation(
        OPERATION,
        "different",
        store.evidence_generation(),
    ))?;
    check!(eq; retained_state(&store)?, before);
    reserved(staged(&store)?.commit_once_at_revision(
        OPERATION,
        "different",
        store.snapshot().revision(),
    ))?;
    check!(eq; retained_state(&store)?, before);
    reserved(staged(&store)?.conditional(OPERATION, "different"))?;
    check!(eq; retained_state(&store)?, before);
    let effect = staged(&store)?.conditional("ordinary-effect", "effect")?;
    reserved(staged(&store)?.commit_once_with_effects(OPERATION, "different", vec![effect]))?;
    check!(eq; retained_state(&store)?, before);
    reserved(store.credit_observations("endpoint", OPERATION))?;
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.endpoint_credit("endpoint")?, 0);
    Ok(())
}
