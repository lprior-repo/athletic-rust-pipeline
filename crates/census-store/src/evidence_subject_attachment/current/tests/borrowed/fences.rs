use super::*;
use crate::{PublicIdentityError, Store};

#[test]
fn borrowed_current_constructor_refuses_a_real_foreign_store_owner() -> TestResult {
    let case = committed("borrowed-foreign-store")?;
    let foreign = Fixture::new()?;
    let foreign_view = foreign.store.snapshot();
    let accepted = accepted(&foreign_view)?;
    let before = retained_state(&case.fixture.store)?;
    let foreign_before = retained_state(&foreign.store)?;
    let view = case.fixture.store.snapshot();
    check!(ne; accepted.store_identity(), &view.store_identity()?);
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &accepted, &mut grant
        ),
        Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    check!(eq; retained_state(&foreign.store)?, foreign_before);
    Ok(())
}

#[test]
fn attachment_only_append_is_a_sequence_mismatch_with_unchanged_store_head_and_v3() -> TestResult {
    let fixture = Fixture::new()?;
    let old_view = fixture.store.snapshot();
    let old_accepted = accepted(&old_view)?;
    let context = Context::synthetic("borrowed-sequence-only")?;
    let inspection = fixture.inspect()?;
    let command = context.command("borrowed-sequence-only", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let current = accepted(&view)?;
    check!(eq; old_accepted.store_identity(), current.store_identity());
    check!(eq; old_accepted.head(), current.head());
    check!(eq; old_accepted.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(eq; old_accepted.qualification_version(), current.qualification_version());
    check!(ne; old_accepted.snapshot_sequence(), view.sequence());
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &old_accepted,
            &mut grant
        ),
        Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
    ));
    poisoned(&mut grant)?;
    let mut original = AttachmentReplayGrantV1::new();
    let old_owner = old_view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &old_accepted,
        &mut original,
    )?;
    check!(old_owner
        .bind_exact_subject(&context.subject, &mut original)?
        .is_none());
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn old_and_fresh_accepted_heads_work_only_with_their_exact_held_views() -> TestResult {
    let case = committed("borrowed-held-heads")?;
    let old_view = case.fixture.store.snapshot();
    let old_accepted = accepted(&old_view)?;
    case.fixture.second_birth()?;
    let view = case.fixture.store.snapshot();
    let current = accepted(&view)?;
    let before = retained_state(&case.fixture.store)?;
    check!(eq; old_accepted.store_identity(), current.store_identity());
    check!(ne; old_accepted.head(), current.head());
    check!(ne; old_accepted.snapshot_sequence(), current.snapshot_sequence());
    for (held, supplied) in [(&old_view, &current), (&view, &old_accepted)] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            held.read_current_evidence_subject_attachment_references_with_accepted_v1(
                supplied, &mut grant
            ),
            Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
        ));
        poisoned(&mut grant)?;
    }
    for (held, supplied) in [(&old_view, &old_accepted), (&view, &current)] {
        let mut grant = AttachmentReplayGrantV1::new();
        let owner = held.read_current_evidence_subject_attachment_references_with_accepted_v1(
            supplied, &mut grant,
        )?;
        let bound = owner
            .bind_exact_subject(&case.context.subject, &mut grant)?
            .ok_or("held subject absent")?;
        bound.require_original_grant(&mut grant)?;
        check!(std::ptr::eq(bound.snapshot(), held));
        check!(eq; bound.accepted_public_head(), supplied.head());
        check!(eq; bound.record(), &case.record);
        check!(eq; bound.receipt(), &case.receipt);
    }
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn historical_v2_accepted_owner_does_not_authorize_a_current_v3_view() -> TestResult {
    let fixture = Fixture::new()?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    {
        let version = PublicIdentityQualificationVersion::RetainedAuthorityContextV2;
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("historical");
        let declaration = crate::public_identity::create_historical_test_fixture(&root, version)?;
        let accepted = Store::replay_historical_public_identity(&root)?;
        check!(eq; accepted.qualification_version(), version);
        check!(eq; accepted.store_identity(), declaration.store_identity());
        check!(ne; accepted.store_identity(), &view.store_identity()?);
        let historical_before = crate::public_identity_tests::support::physical_state(&root)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            view.read_current_evidence_subject_attachment_references_with_accepted_v1(
                &accepted, &mut grant
            ),
            Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
        ));
        poisoned(&mut grant)?;
        check!(eq; crate::public_identity_tests::support::physical_state(&root)?, historical_before);
    }
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_completes_ledger_receipt_validation_before_owner_comparison() -> TestResult
{
    let case = committed("borrowed-ledger-before-mismatch")?;
    let foreign = Fixture::new()?;
    let foreign_view = foreign.store.snapshot();
    let accepted = accepted(&foreign_view)?;
    case.fixture
        .store
        .receipts
        .remove(case.receipt.operation.as_bytes())?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &accepted, &mut grant
        ),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "missing attachment receipt"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_reads_the_exact_held_head_before_issuing_authority() -> TestResult {
    let fixture = Fixture::new()?;
    let old_view = fixture.store.snapshot();
    let accepted = accepted(&old_view)?;
    fixture.store.meta.remove("public_identity:v1:head")?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &accepted, &mut grant
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "missing head"
        )))
    ));
    poisoned(&mut grant)?;
    let mut original = AttachmentReplayGrantV1::new();
    let owner = old_view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted,
        &mut original,
    )?;
    check!(std::ptr::eq(owner.accepted.view(), &accepted));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
