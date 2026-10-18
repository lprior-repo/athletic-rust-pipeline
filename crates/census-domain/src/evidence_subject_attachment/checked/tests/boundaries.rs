use super::*;

#[test]
fn command_and_fresh_decode_issuance_have_exact_last_step_boundaries() -> TestResult {
    let dto = command("issuance boundary")?;
    let mut exact = AttachmentReplayGrantV1::new();
    leave_steps(&mut exact, CHECK_WORK)?;
    let checked = check_attachment_command_with_grant(dto.clone(), &mut exact)?;
    check!(eq; checked.command(), &dto);
    check!(eq; exact.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    let mut short = AttachmentReplayGrantV1::new();
    leave_steps(&mut short, COMMAND_WORK)?;
    capacity(check_attachment_command_with_grant(dto.clone(), &mut short))?;
    check!(eq; short.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    poisoned(&short)?;
    let record = seal(dto)?;
    let raw = encode_reviewed_reference(&record, MAXIMUM)?;
    let mut exact = AttachmentReplayGrantV1::new();
    leave_steps(&mut exact, DECODE_WORK)?;
    check!(eq; decode_checked_reviewed_reference_with_grant(&raw, MAXIMUM, &mut exact)?.record(), &record);
    check!(eq; exact.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    let mut short = AttachmentReplayGrantV1::new();
    leave_steps(&mut short, DECODE_WORK - 1)?;
    capacity(decode_checked_reviewed_reference_with_grant(
        &raw, MAXIMUM, &mut short,
    ))?;
    check!(eq; short.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    poisoned(&short)?;
    Ok(())
}

#[test]
fn exhausted_origin_check_preserves_capacity_before_foreign_origin_or_bytes() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    let checked = check_attachment_command_with_grant(command("origin admission")?, &mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    leave_steps(&mut foreign, 0)?;
    capacity(checked.approval_bytes_with_grant(MAXIMUM, &mut foreign))?;
    check!(eq; foreign.usage().encoded_bytes(), 0);
    poisoned(&foreign)?;
    check!(matches!(
        checked.approval_bytes_with_grant(MAXIMUM, &mut foreign),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    original.ensure_available()?;
    Ok(())
}

#[test]
fn second_push_check_exhaustion_keeps_unbound_collection_empty() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let record = draft("push admission", &mut grant)?;
    let mut records = CheckedReviewedReferenceRecordsV1::new();
    leave_steps(&mut grant, 1)?;
    capacity(records.push_with_grant(record, &mut grant))?;
    check!(records.records().is_empty());
    check!(eq; grant.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    poisoned(&grant)?;
    let mut fresh = AttachmentReplayGrantV1::new();
    replay(&records, &mut fresh)?;
    Ok(())
}

#[test]
fn extraction_issuance_exhaustion_keeps_pop_and_remove_atomic() -> TestResult {
    let dto = seal(command("extraction issuance")?)?;
    let mut pop_grant = AttachmentReplayGrantV1::new();
    let mut pop_records = collect(std::slice::from_ref(&dto), &mut pop_grant)?;
    leave_steps(&mut pop_grant, 1)?;
    capacity(pop_records.pop_with_grant(&mut pop_grant))?;
    check!(eq; pop_records.records(), std::slice::from_ref(&dto));
    poisoned(&pop_grant)?;
    let mut remove_grant = AttachmentReplayGrantV1::new();
    let mut remove_records = collect(std::slice::from_ref(&dto), &mut remove_grant)?;
    leave_steps(&mut remove_grant, 1)?;
    capacity(remove_records.remove_with_grant(0, &mut remove_grant))?;
    check!(eq; remove_records.records(), std::slice::from_ref(&dto));
    poisoned(&remove_grant)?;
    Ok(())
}

#[test]
fn roundtrip_equality_and_issuance_admission_use_the_original_grant() -> TestResult {
    for remaining in [2, ROUNDTRIP_WORK - 1] {
        let mut grant = AttachmentReplayGrantV1::new();
        let record = draft("roundtrip bounded passes", &mut grant)?;
        let original = record.record().clone();
        let raw = encode_reviewed_reference(&original, MAXIMUM)?;
        leave_steps(&mut grant, remaining)?;
        capacity(record.decode_roundtrip_with_grant(&raw, MAXIMUM, &mut grant))?;
        check!(eq; record.record(), &original);
        check!(eq; grant.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
        poisoned(&grant)?;
    }
    let mut exact = AttachmentReplayGrantV1::new();
    let record = draft("exact roundtrip work", &mut exact)?;
    let raw = encode_reviewed_reference(record.record(), MAXIMUM)?;
    leave_steps(&mut exact, ROUNDTRIP_WORK)?;
    check!(eq; record.decode_roundtrip_with_grant(&raw, MAXIMUM, &mut exact)?.record(), record.record());
    check!(eq; exact.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    Ok(())
}

#[test]
fn checked_encoding_keeps_existing_record_byte_debit_and_poison() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let record = draft("retained encoded bytes", &mut grant)?;
    let raw = encode_reviewed_reference(record.record(), MAXIMUM)?;
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(raw.len())
        .and_then(|n| n.checked_add(1))
        .ok_or("byte padding overflow")?;
    grant.charge_bytes(padding)?;
    check!(matches!(
        record.encode_with_grant(MAXIMUM, &mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq; grant.usage().encoded_bytes(), padding);
    poisoned(&grant)?;
    Ok(())
}
