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
    let case = committed("borrowed-current-parity")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let history = serde_json::to_vec(accepted.history_records())?;
    let mut historical = AttachmentReplayGrantV1::new();
    view.read_evidence_subject_attachment_references_v1(&mut historical)?;
    let mut owned_grant = AttachmentReplayGrantV1::new();
    let owned = view.read_current_evidence_subject_attachment_references_v1(&mut owned_grant)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    check!(std::ptr::eq(owner.accepted.view(), &accepted));
    check!(std::ptr::eq(owner.snapshot(), &view));
    check!(eq; grant.usage().traversal_steps(), historical.usage().traversal_steps().checked_add(2).ok_or("constructor work")?);
    check!(eq; grant.usage().traversal_steps(), owned_grant.usage().traversal_steps());
    check!(eq; grant.usage().encoded_bytes(), historical.usage().encoded_bytes());
    check!(eq; grant.usage().encoded_bytes(), owned_grant.usage().encoded_bytes());
    let steps = grant.usage().traversal_steps();
    let bytes = grant.usage().encoded_bytes();
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
    check!(eq; grant.usage().traversal_steps(), steps.checked_add(30).ok_or("query work")?);
    let projection = binding_bytes(&case)?;
    check!(eq; grant.usage().encoded_bytes(), bytes.checked_add(projection).and_then(|n| n.checked_add(case.group_bytes)).ok_or("query bytes")?);
    bound.require_original_grant(&mut grant)?;
    original.require_original_grant(&mut owned_grant)?;
    check!(eq; grant.usage().traversal_steps(), steps.checked_add(47).ok_or("consume work")?);
    check!(eq; grant.usage().encoded_bytes(), bytes.checked_add(projection).and_then(|n| n.checked_add(case.group_bytes)).and_then(|n| n.checked_add(projection)).ok_or("consume bytes")?);
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
    check!(eq; original.usage().traversal_steps(), 12);
    check!(eq; original.usage().encoded_bytes(), 0);
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(owner
        .bind_exact_subject(&context.subject, &mut foreign)?
        .is_none());
    check!(eq; foreign.usage().traversal_steps(), 3);
    check!(eq; foreign.usage().encoded_bytes(), 0);
    foreign.charge_steps(0)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
