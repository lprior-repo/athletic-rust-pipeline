use super::*;
use crate::Store;

#[test]
fn current_reader_refuses_a_missing_protected_receipt_before_subject_authority() -> TestResult {
    let case = committed("current-missing-receipt")?;
    case.fixture
        .store
        .receipts
        .remove(case.receipt.operation.as_bytes())?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "missing attachment receipt"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn current_reader_refuses_an_orphan_attachment_receipt_in_the_complete_prefix() -> TestResult {
    let case = committed("current-orphan-receipt")?;
    case.fixture.store.journal.remove(Store::journal_key(
        "evidence_subject",
        "attachment/v1/record/00000000000000000001",
    ))?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "orphan attachment receipt"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn current_reader_exhausts_later_protected_rows_before_issuing_an_earlier_match() -> TestResult {
    let case = committed("current-malformed-later-root")?;
    case.fixture.store.journal.insert(
        Store::journal_key(
            "evidence_subject",
            "attachment/v1/record/00000000000000000002",
        ),
        b"{",
    )?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::Decode { source, .. }) if source.classify() == serde_json::error::Category::Eof)
    );
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn current_reader_cannot_promote_a_store_without_an_accepted_v3_identity_view() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), 8_388_608)?;
    store.store_identity()?;
    let before = retained_state(&store)?;
    let view = store.snapshot();
    check!(view.accepted_public_identity_snapshot()?.is_none());
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
