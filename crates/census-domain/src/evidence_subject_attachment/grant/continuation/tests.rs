use super::*;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn continuation_keeps_original_charges_and_accepts_only_original_grant() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    original.charge_bytes(31)?;
    let checkpoint = original.checkpoint()?;
    let before = original.usage();
    original.charge_bytes(17)?;
    checkpoint.require(&mut original)?;
    check!(eq; original.usage().encoded_bytes(), 48);
    check!(original.usage().traversal_steps() > before.traversal_steps());
    Ok(())
}

#[test]
fn replenished_foreign_grant_does_not_satisfy_checkpoint() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    original.charge_bytes(31)?;
    let checkpoint = original.checkpoint()?;
    let mut foreign = AttachmentReplayGrantV1::new();
    foreign.charge_bytes(128)?;
    foreign.charge_steps(128)?;
    let _foreign_checkpoint = foreign.checkpoint()?;
    check!(checkpoint.require(&mut foreign).is_err());
    check!(foreign.ensure_available().is_err());
    checkpoint.require(&mut original)?;
    Ok(())
}

#[test]
fn poisoned_grant_cannot_issue_or_resume_continuation() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let checkpoint = grant.checkpoint()?;
    grant.poison();
    check!(matches!(
        grant.checkpoint(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        checkpoint.require(&mut grant),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn authority_check_rejects_temporary_swap_before_original_is_restored() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let checkpoint = grant.checkpoint()?;
    let original = std::mem::replace(&mut grant, AttachmentReplayGrantV1::new());
    check!(checkpoint.require(&mut grant).is_err());
    check!(grant.ensure_available().is_err());
    grant = original;
    checkpoint.require(&mut grant)?;
    Ok(())
}

#[test]
fn checkpoint_and_continuation_each_charge_existing_exact_step_cap() -> TestResult {
    let limit = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(
        limit
            .checked_sub(2)
            .ok_or(EvidenceSubjectAttachmentError::Overflow)?,
    )?;
    let checkpoint = grant.checkpoint()?;
    checkpoint.require(&mut grant)?;
    check!(eq; grant.usage().traversal_steps(), limit);
    check!(matches!(
        checkpoint.require(&mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            ..
        })
    ));
    check!(grant.ensure_available().is_err());
    Ok(())
}
