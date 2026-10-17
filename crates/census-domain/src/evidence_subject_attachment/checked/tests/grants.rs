use super::*;

#[test]
fn moved_original_grant_preserves_command_proof_and_exact_approval_cost() -> TestResult {
    let dto = command("checked moved grant")?;
    let expected = dto.approval_bytes(MAXIMUM)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(7)?;
    let checked = check_attachment_command_with_grant(dto.clone(), &mut grant)?;
    check!(eq; checked.command(), &dto);
    check!(eq; grant.usage().traversal_steps(), 7 + CHECK_WORK);
    let mut moved = grant;
    check!(eq; checked.approval_bytes_with_grant(MAXIMUM, &mut moved)?, expected);
    check!(eq; moved.usage().traversal_steps(), 7 + CHECK_WORK + 3);
    check!(eq; moved.usage().encoded_bytes(), expected.len());
    Ok(())
}

#[test]
fn replacing_grant_cannot_reuse_proof_but_moved_original_still_can() -> TestResult {
    let mut slot = AttachmentReplayGrantV1::new();
    let checked = check_attachment_command_with_grant(command("replace grant")?, &mut slot)?;
    let mut original = std::mem::take(&mut slot);
    invalid(
        checked.approval_bytes_with_grant(MAXIMUM, &mut slot),
        "checked command grant differs",
    )?;
    poisoned(&slot)?;
    check!(eq; slot.usage().traversal_steps(), 1);
    check!(eq; slot.usage().encoded_bytes(), 0);
    check!(eq; checked.approval_bytes_with_grant(MAXIMUM, &mut original)?, checked.command().approval_bytes(MAXIMUM)?);
    check!(eq; original.usage().traversal_steps(), CHECK_WORK + 3);
    Ok(())
}

#[test]
fn independently_anchored_grant_cannot_consume_another_commands_proof() -> TestResult {
    let mut left = AttachmentReplayGrantV1::new();
    let mut right = AttachmentReplayGrantV1::new();
    let checked = check_attachment_command_with_grant(command("same immutable DTO")?, &mut left)?;
    let independently_checked =
        check_attachment_command_with_grant(checked.command().clone(), &mut right)?;
    invalid(
        checked.approval_bytes_with_grant(MAXIMUM, &mut right),
        "checked command grant differs",
    )?;
    poisoned(&right)?;
    check!(eq; right.usage().traversal_steps(), CHECK_WORK + 1);
    check!(eq; independently_checked.command(), checked.command());
    left.ensure_available()?;
    check!(eq; left.usage().traversal_steps(), CHECK_WORK);
    Ok(())
}

#[test]
fn proof_outlives_original_grant_without_becoming_valid_for_a_new_grant() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    let checked = check_attachment_command_with_grant(command("retained anchor")?, &mut original)?;
    drop(original);
    for _ in 0..4 {
        let mut fresh = AttachmentReplayGrantV1::new();
        let other = check_attachment_command_with_grant(command("other anchor")?, &mut fresh)?;
        invalid(
            checked.approval_bytes_with_grant(MAXIMUM, &mut fresh),
            "checked command grant differs",
        )?;
        poisoned(&fresh)?;
        check!(eq; other.command().operation, text("other anchor")?);
    }
    Ok(())
}

#[test]
fn raw_command_extraction_requires_complete_validation_again() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    let checked = check_attachment_command_with_grant(command("extract command")?, &mut original)?;
    let mut dto = checked.into_command();
    dto.version = 2;
    let mut fresh = AttachmentReplayGrantV1::new();
    invalid(
        check_attachment_command_with_grant(dto, &mut fresh),
        "command schema",
    )?;
    check!(eq; fresh.usage().traversal_steps(), 1);
    poisoned(&fresh)?;
    original.ensure_available()?;
    Ok(())
}

#[test]
fn fresh_invalid_command_and_record_never_return_checked_owners() -> TestResult {
    let mut dto = command("invalid fresh owner")?;
    dto.version = 2;
    let mut grant = AttachmentReplayGrantV1::new();
    invalid(
        check_attachment_command_with_grant(dto.clone(), &mut grant),
        "command schema",
    )?;
    poisoned(&grant)?;
    let raw = serde_json::to_vec(&seal(dto)?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    invalid(
        decode_checked_reviewed_reference_with_grant(&raw, MAXIMUM, &mut grant),
        "command schema",
    )?;
    poisoned(&grant)?;
    let mut record = seal(command("invalid fresh record shell")?)?;
    record.approval_digest = digest("invalid fresh approval digest")?;
    let raw = serde_json::to_vec(&record)?;
    let mut grant = AttachmentReplayGrantV1::new();
    invalid(
        decode_checked_reviewed_reference_with_grant(&raw, MAXIMUM, &mut grant),
        "reviewed record digest or time",
    )?;
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn checked_owners_retain_send_and_sync() {
    fn require<T: Send + Sync>() {}
    require::<CheckedAttachmentCommandV1>();
    require::<CheckedReviewedReferenceRecordV1>();
    require::<CheckedReviewedReferenceRecordsV1>();
}
