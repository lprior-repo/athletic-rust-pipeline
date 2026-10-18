use super::*;

#[test]
fn earlier_committed_at_is_rejected_and_poisons_grant() -> TestResult {
    let value = command("controlled-time")?;
    let earlier = value
        .reviewed_at
        .checked_sub_signed(chrono::Duration::seconds(1))
        .ok_or("earlier review boundary")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let checked = CheckedResultSchoolCommandV1::check(value, &mut grant)?;
    check!(matches!(
        prepare_result_school_record_v1(checked, earlier, &mut grant),
        Err(ResultSchoolCorrespondenceError::Invalid(_))
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
