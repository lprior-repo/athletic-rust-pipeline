use super::*;

pub(super) const MAXIMUM: usize = ReplayLimits::SMALL.total_text_bytes;
pub(super) const COMMAND_WORK: usize = 107;
pub(super) const CHECK_WORK: usize = COMMAND_WORK + 1;
pub(super) const DECODE_WORK: usize = COMMAND_WORK + 10;
pub(super) const ENCODE_WORK: usize = 11;
pub(super) const ROUNDTRIP_WORK: usize = 12;

pub(super) fn invalid<T>(result: Result<T>, expected: &'static str) -> TestResult {
    check!(
        matches!(result, Err(EvidenceSubjectAttachmentError::Invalid(actual)) if actual == expected)
    );
    Ok(())
}

pub(super) fn poisoned(grant: &AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

pub(super) fn capacity<T>(result: Result<T>) -> TestResult {
    check!(matches!(
        result,
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8_193,
            available: 8_192
        })
    ));
    Ok(())
}

pub(super) fn leave_steps(grant: &mut AttachmentReplayGrantV1, remaining: usize) -> TestResult {
    let padding = ReplayLimits::SMALL
        .traversal_steps
        .checked_sub(grant.usage().traversal_steps())
        .and_then(|value| value.checked_sub(remaining))
        .ok_or("checked test padding underflow")?;
    grant.charge_steps(padding)?;
    Ok(())
}

pub(super) fn draft(
    label: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<CheckedReviewedReferenceRecordV1> {
    Ok(
        check_attachment_command_with_grant(command(label)?, grant)?
            .into_draft_with_grant(grant)?,
    )
}

pub(super) fn collect(
    records: &[ReviewedReferenceRecordV1],
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<CheckedReviewedReferenceRecordsV1> {
    let mut checked = CheckedReviewedReferenceRecordsV1::new();
    for record in records {
        let bytes = encode_reviewed_reference(record, MAXIMUM)?;
        let record = decode_checked_reviewed_reference_with_grant(&bytes, MAXIMUM, grant)?;
        checked.push_with_grant(record, grant)?;
    }
    Ok(checked)
}

pub(super) fn replay(
    records: &CheckedReviewedReferenceRecordsV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<()> {
    let receipts = receipts(records.records());
    let expected = calculate_reviewed_reference_inventory_with_grant(
        records.records(),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    replay_checked_reviewed_references_with_grant(
        &StoreIdentity::parse(&"1".repeat(64))?,
        records,
        &receipts,
        &expected,
        grant,
    )
}
