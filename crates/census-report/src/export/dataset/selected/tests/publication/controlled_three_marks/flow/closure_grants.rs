use super::super::source::{self, TestResult};
use crate::export::qualification::participation::archive;
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, AttachmentReplayUsageV1, EvidenceSubjectAttachmentError,
};
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError};

fn replenished_foreign_grant(
    floor: AttachmentReplayUsageV1,
) -> TestResult<AttachmentReplayGrantV1> {
    let steps = floor
        .traversal_steps()
        .checked_add(1)
        .ok_or("replenished traversal steps overflow")?;
    let bytes = floor
        .encoded_bytes()
        .checked_add(1)
        .ok_or("replenished encoded bytes overflow")?;
    check!(steps <= AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    check!(bytes <= AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    let mut foreign = AttachmentReplayGrantV1::new();
    foreign.checkpoint()?;
    foreign.charge_steps(steps)?;
    foreign.charge_bytes(bytes)?;
    Ok(foreign)
}

fn poison_by_step_capacity(grant: &mut AttachmentReplayGrantV1) -> TestResult {
    let usage = grant.usage();
    let remaining = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
        .checked_sub(usage.traversal_steps())
        .ok_or("original grant usage already exceeds the traversal ceiling")?;
    let over = remaining
        .checked_add(1)
        .ok_or("over-budget traversal charge overflow")?;
    let needed = usage
        .traversal_steps()
        .checked_add(over)
        .ok_or("traversal need overflow")?;
    let error = grant
        .charge_steps(over)
        .err()
        .ok_or("over-budget traversal charge was admitted")?;
    check!(matches!(
        error,
        EvidenceSubjectAttachmentError::Capacity {
            needed: observed,
            available, ..
        } if observed == needed && available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn retained_attachment_closure_rejects_foreign_and_poisoned_grants() -> TestResult {
    let installed = source::install()?;
    let (_current_school, _reviewed_point) = super::reviewed_inputs(&installed)?;
    let snapshot = installed.store.snapshot();
    let identity = snapshot.freeze_public_identity_v2()?;
    let store_identity = snapshot.store_identity()?;
    let sequence = snapshot.sequence();
    let mut original = AttachmentReplayGrantV1::new();
    let closure = archive::capture_school(&snapshot, &identity, &mut original)?;
    let replayed = closure.replay(store_identity.as_str(), sequence, &mut original)?;

    let continuation_floor = original.usage();
    replayed.retained_attachment_closure(&mut original)?;

    let mut foreign = replenished_foreign_grant(continuation_floor)?;
    let foreign_usage = foreign.usage();
    check!(foreign_usage.traversal_steps() >= continuation_floor.traversal_steps());
    check!(foreign_usage.encoded_bytes() >= continuation_floor.encoded_bytes());
    check!(foreign_usage.traversal_steps() < AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    let error = replayed
        .retained_attachment_closure(&mut foreign)
        .err()
        .ok_or("replenished foreign grant was admitted")?;
    check!(matches!(
        error,
        StoreError::EvidenceSubjectAttachment(EvidenceSubjectAttachmentStoreError::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        ))
    ));
    check!(foreign.usage().traversal_steps() >= continuation_floor.traversal_steps());
    check!(foreign.usage().encoded_bytes() >= continuation_floor.encoded_bytes());
    check!(matches!(
        foreign.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));

    original.ensure_available()?;
    poison_by_step_capacity(&mut original)?;
    let poisoned_usage = original.usage();
    let error = replayed
        .retained_attachment_closure(&mut original)
        .err()
        .ok_or("poisoned original grant was admitted")?;
    check!(matches!(
        error,
        StoreError::EvidenceSubjectAttachment(EvidenceSubjectAttachmentStoreError::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        ))
    ));
    check!(eq; original.usage(), poisoned_usage);
    check!(matches!(
        original.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    source::require_originals(&installed.store, &installed.source_records)?;
    Ok(())
}
