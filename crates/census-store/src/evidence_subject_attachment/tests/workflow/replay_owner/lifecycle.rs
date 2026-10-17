use super::*;
use crate::evidence_subject_attachment::AttachmentInspectionRequestV1;

#[test]
fn reopened_first_and_later_replay_keep_exact_original_receipts_and_all_physical_state(
) -> TestResult {
    let case = committed("replay-owner-reopened-first")?;
    let (_, later_receipt, later) = append(&case.fixture, "replay-owner-reopened-later")?;
    let before = retained_state(&case.fixture.store)?;
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = case.fixture;
    drop(writer);
    drop(store);
    let (reopened, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; retained_state(&reopened)?, before);
    unchanged_replay(&reopened, &case.record, &case.receipt)?;
    unchanged_replay(&reopened, &later, &later_receipt)?;
    let view = reopened.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; retained.records(), &[case.record, later]);
    check!(eq; retained.receipts(), &[case.receipt, later_receipt]);
    check!(eq; active_operations(&retained)?.len(), 2);
    Ok(())
}

#[test]
fn original_receipt_replay_after_revocation_preserves_inactive_history() -> TestResult {
    let case = committed("replay-owner-original-before-revoke")?;
    let inspection = case.fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Revoke {
            prior: case.record.reference(),
        },
    )?;
    let revoke = next_command(&case.record, "replay-owner-revocation", &inspection)?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        revoke,
        Preparation::New {
            inspection,
            context: &[],
        },
    )?;
    let revoke_receipt = written(apply(&case.fixture.store, prepared)?)?;
    unchanged_replay(&case.fixture.store, &case.record, &case.receipt)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; retained.records().first(), Some(&case.record));
    check!(eq; retained.receipts(), &[case.receipt, revoke_receipt]);
    check!(active_operations(&retained)?.is_empty());
    Ok(())
}

#[test]
fn replay_owner_changed_inventory_falls_back_to_exact_original_record_without_appending(
) -> TestResult {
    let case = committed("replay-owner-race-first")?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let (_, later_receipt, later) = append(&case.fixture, "replay-owner-race-later")?;
    let before = retained_state(&case.fixture.store)?;
    check!(eq; apply(&case.fixture.store, prepared)?, Application::Repeated(case.receipt.clone()));
    check!(eq; retained_state(&case.fixture.store)?, before);
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records(), &[case.record, later]);
    check!(eq; read.receipts(), &[case.receipt, later_receipt]);
    Ok(())
}

#[test]
fn replay_owner_still_requires_the_exact_explicit_approval_before_refresh() -> TestResult {
    let case = committed("replay-owner-explicit-approval")?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let digest = prepared.approval_digest().clone();
    let before = retained_state(&case.fixture.store)?;
    check!(matches!(
        case.fixture.store.apply_evidence_subject_attachment_v1(
            prepared,
            b"wrong explicit approval",
            &digest
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("explicit attachment approval differs")
        ))
    ));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}
