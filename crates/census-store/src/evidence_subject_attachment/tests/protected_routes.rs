use crate::evidence_subject_attachment::{EvidenceSubjectAttachmentStoreError, PHASE};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Store, StoreError, StoreResult};

const KEY: &str = "attachment/v1/record/00000000000000000001";
const OPERATION: &str = "evidence_subject_attachment_v1:protected-route";

fn fixture() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store
        .journal
        .insert(Store::journal_key(PHASE, KEY), b"retained root bytes")?;
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
        Err(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::ReservedWrite
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
fn direct_and_locked_journal_paths_refuse_reserved_phase_and_nul_alias() -> TestResult {
    let (_directory, store) = fixture()?;
    let before = retained_state(&store)?;
    for phase in [PHASE, "evidence_subject\0attachment/v1/record"] {
        reserved(store.journal_done(phase, KEY, &false))?;
        check!(eq; retained_state(&store)?, before);
        {
            let _guard = store.lock_appends();
            reserved(store.journal_done_locked(phase, KEY, &false))?;
        }
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn batch_and_derived_journal_routes_cannot_overwrite_protected_root() -> TestResult {
    let (_directory, store) = fixture()?;
    let before = retained_state(&store)?;
    for phase in [PHASE, "evidence_subject\0attachment/v1/record"] {
        let mut batch = staged(&store)?;
        reserved(batch.journal_done(phase, KEY, &false))?;
        drop(batch);
        check!(eq; retained_state(&store)?, before);
    }
    let mut stage = store.stage_derived()?;
    let after_reservation = retained_state(&store)?;
    reserved(stage.journal_done_once(PHASE, KEY, &false))?;
    check!(eq; retained_state(&store)?, after_reservation);
    check!(matches!(
        stage.journal_done_once("evidence_subject\0attachment/v1/record", KEY, &false),
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

#[test]
fn neighboring_generic_names_keep_their_existing_write_and_credit_behavior() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done("evidence_subject_other", "direct", &true)?;
    let mut batch = store.write_batch();
    batch.journal_done("evidence_subject_other", "batch", &false)?;
    let operation = "evidence_subject_attachment_v10:ordinary";
    check!(batch.commit_once(operation, "ordinary")?.written());
    let view = store.snapshot();
    check!(eq; view.journal_payload("evidence_subject_other", "direct")?, Some(serde_json::json!(true)));
    check!(eq; view.journal_payload("evidence_subject_other", "batch")?, Some(serde_json::json!(false)));
    check!(eq; store.credit_observations("ordinary-endpoint", operation)?.credited, 0);
    check!(
        store
            .receipt(operation)?
            .ok_or("ordinary receipt missing")?
            .credited
    );
    Ok(())
}
