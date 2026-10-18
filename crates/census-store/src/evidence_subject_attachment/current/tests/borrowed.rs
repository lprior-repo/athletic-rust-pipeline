use super::*;
use crate::AcceptedPublicIdentitySnapshot;

mod fences;
mod grant;

fn accepted(view: &StoreSnapshot<'_>) -> TestResult<AcceptedPublicIdentitySnapshot> {
    Ok(view
        .accepted_public_identity_snapshot()?
        .ok_or("accepted view absent")?)
}

#[test]
fn borrowed_current_owner_reuses_the_exact_accepted_view_and_matches_owned_constructor(
) -> TestResult {
    let case = committed("borrowed-owner-equivalence")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let history = serde_json::to_vec(accepted.history_records())?;
    let mut owned_grant = AttachmentReplayGrantV1::new();
    let owned = view.read_current_evidence_subject_attachment_references_v1(&mut owned_grant)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    check!(std::ptr::eq(owner.accepted.view(), &accepted));
    check!(std::ptr::eq(owner.snapshot(), &view));
    check!(eq; grant.usage().traversal_steps(), owned_grant.usage().traversal_steps());
    check!(eq; grant.usage().encoded_bytes(), owned_grant.usage().encoded_bytes());
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("borrowed subject absent")?;
    let original = owned
        .bind_exact_subject(&case.context.subject, &mut owned_grant)?
        .ok_or("owned subject absent")?;
    check!(eq; bound.record(), original.record());
    check!(eq; bound.receipt(), original.receipt());
    check!(eq; bound.inventory(), original.inventory());
    check!(eq; bound.accepted_public_head(), accepted.head());
    bound.require_original_grant(&mut grant)?;
    original.require_original_grant(&mut owned_grant)?;
    check!(eq; grant.usage().traversal_steps(), owned_grant.usage().traversal_steps());
    check!(eq; grant.usage().encoded_bytes(), owned_grant.usage().encoded_bytes());
    check!(eq; serde_json::to_vec(accepted.history_records())?, history);
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn empty_borrowed_current_owner_preserves_unbound_absence_on_a_fresh_grant() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("borrowed-empty")?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted,
        &mut original,
    )?;
    check!(std::ptr::eq(owner.accepted.view(), &accepted));
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(owner
        .bind_exact_subject(&context.subject, &mut foreign)?
        .is_none());
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
