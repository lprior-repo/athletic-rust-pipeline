use super::*;
use crate::evidence_subject_attachment::tests::workflow::membership::ready_for_merge;

#[test]
fn unrelated_public_head_advance_preserves_current_exact_group_and_old_held_view() -> TestResult {
    let case = committed("current-unrelated-head")?;
    let old_view = case.fixture.store.snapshot();
    let mut old_grant = AttachmentReplayGrantV1::new();
    let old_owner =
        old_view.read_current_evidence_subject_attachment_references_v1(&mut old_grant)?;
    let old_head = case.fixture.writer.head(&case.fixture.store)?;
    case.fixture.second_birth()?;
    let new_head = case.fixture.writer.head(&case.fixture.store)?;
    check!(ne; new_head, old_head);
    let old_bound = old_owner
        .bind_exact_subject(&case.context.subject, &mut old_grant)?
        .ok_or("old held subject absent")?;
    old_bound.require_original_grant(&mut old_grant)?;
    check!(eq; old_bound.accepted_public_head(), &old_head);
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("unchanged group absent")?;
    bound.require_original_grant(&mut grant)?;
    check!(eq; bound.accepted_public_head(), &new_head);
    check!(eq; bound.target(), old_bound.target());
    check!(eq; bound.record(), old_bound.record());
    Ok(())
}

fn changed_target(index: usize) -> TestResult {
    let fixture = ready_for_merge()?;
    let birth = fixture.births.get(index).ok_or("controlled birth absent")?;
    let context = Context::synthetic("current-public-target-change")?;
    let inspection = inspect_first(&fixture.store, birth.public_id())?;
    let command = context.command("current-public-target-change", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let old_view = fixture.store.snapshot();
    let mut old_grant = AttachmentReplayGrantV1::new();
    let old_owner =
        old_view.read_current_evidence_subject_attachment_references_v1(&mut old_grant)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "current-target-reviewed-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let before = retained_state(&fixture.store)?;
    let old_bound = old_owner
        .bind_exact_subject(&context.subject, &mut old_grant)?
        .ok_or("old held group absent")?;
    old_bound.require_original_grant(&mut old_grant)?;
    check!(std::ptr::eq(old_bound.snapshot(), &old_view));
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(matches!(
        owner.bind_exact_subject(&context.subject, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn current_canonical_membership_change_refuses_the_old_group_at_the_new_view() -> TestResult {
    changed_target(0)
}

#[test]
fn current_redirect_refuses_the_old_public_target_at_the_new_view() -> TestResult {
    changed_target(1)
}
