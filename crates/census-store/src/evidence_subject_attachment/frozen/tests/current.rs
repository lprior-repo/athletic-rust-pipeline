use crate::evidence_subject_attachment::frozen::{
    replay_frozen_evidence_subject_attachment_references_v1 as replay,
    FrozenEvidenceSubjectAttachmentBundleV1 as Bundle,
};
use crate::evidence_subject_attachment::tests::workflow::fixture::{
    apply, first_record, prepare, written, Context, Fixture,
};
use crate::evidence_subject_attachment::EvidenceSubjectAttachmentStoreError as Error;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{AcceptedPublicIdentitySnapshot, Receipt, Store, StoreError};
use census_domain::evidence_subject_attachment::*;

mod fences;
mod support;
use support::*;

#[test]
fn authentic_held_freeze_replay_and_exact_binding_preserve_the_full_current_owner() -> TestResult {
    let (fixture, context, receipt) = committed("frozen-current-roundtrip")?;
    let before = retained_state(&fixture.store)?;
    let record = first_record(&fixture.store)?;
    let (bundle, accepted) = capture(&fixture.store)?;
    let original_bytes = bundle.bytes().to_vec();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = replay(&bundle, &accepted, &mut grant)?;
    check!(eq; owner.store_identity(), accepted.store_identity());
    check!(eq; owner.accepted_public_head(), accepted.head());
    check!(eq; owner.snapshot_sequence(), accepted.snapshot_sequence());
    check!(eq; owner.inventory().records, 1);
    let caller = context.subject.clone();
    let bound = owner
        .bind_exact_subject(&caller, &mut grant)?
        .ok_or("frozen subject absent")?;
    drop(caller);
    check!(eq; bound.subject(), &context.subject);
    check!(eq; bound.record(), &record);
    check!(eq; bound.target(), &record.command.target);
    check!(eq; bound.receipt(), &receipt);
    check!(eq; bound.inventory(), owner.inventory());
    check!(eq; bound.store_identity(), accepted.store_identity());
    check!(eq; bound.accepted_public_head(), accepted.head());
    check!(eq; bound.snapshot_sequence(), accepted.snapshot_sequence());
    check!(std::ptr::eq(bound.source(), owner.source()));
    bound.require_original_grant(&mut grant)?;
    check!(eq; bundle.bytes(), original_bytes.as_slice());
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn frozen_query_and_consumer_reject_replacement_grants_while_original_remains_usable() -> TestResult
{
    let (fixture, context, _) = committed("frozen-current-grant")?;
    let before = retained_state(&fixture.store)?;
    let (bundle, accepted) = capture(&fixture.store)?;
    let mut original = AttachmentReplayGrantV1::new();
    let owner = replay(&bundle, &accepted, &mut original)?;
    let mut replacement = AttachmentReplayGrantV1::new();
    check!(matches!(
        owner.bind_exact_subject(&context.subject, &mut replacement),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut replacement)?;
    original.ensure_available()?;
    let bound = owner
        .bind_exact_subject(&context.subject, &mut original)?
        .ok_or("original subject absent")?;
    let mut replacement = AttachmentReplayGrantV1::new();
    let usage = original.usage();
    replacement.charge_bytes(usage.encoded_bytes())?;
    replacement.charge_steps(usage.traversal_steps())?;
    check!(matches!(
        bound.require_original_grant(&mut replacement),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut replacement)?;
    bound.require_original_grant(&mut original)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
