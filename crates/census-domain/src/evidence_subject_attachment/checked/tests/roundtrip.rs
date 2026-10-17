use super::*;

#[test]
fn checked_codec_keeps_exact_golden_bytes_and_independent_pass_counts() -> TestResult {
    let dto = seal(command("checked codec golden")?)?;
    let raw = encode_reviewed_reference(&dto, MAXIMUM)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let record = decode_checked_reviewed_reference_with_grant(&raw, MAXIMUM, &mut grant)?;
    check!(eq; record.record(), &dto);
    check!(eq; grant.usage().traversal_steps(), DECODE_WORK);
    check!(eq; grant.usage().encoded_bytes(), raw.len());
    check!(eq; record.encode_with_grant(MAXIMUM, &mut grant)?, raw);
    check!(eq; grant.usage().traversal_steps(), DECODE_WORK + ENCODE_WORK);
    check!(eq; grant.usage().encoded_bytes(), raw.len() * 2);
    let decoded = record.decode_roundtrip_with_grant(&raw, MAXIMUM, &mut grant)?;
    check!(eq; decoded.record(), &dto);
    check!(eq; grant.usage().traversal_steps(), DECODE_WORK + ENCODE_WORK + ROUNDTRIP_WORK);
    check!(eq; grant.usage().encoded_bytes(), raw.len() * 3);
    check!(eq; decoded.into_record(), dto);
    Ok(())
}

#[test]
fn fully_resealed_different_command_cannot_inherit_roundtrip_proof() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let record = draft("roundtrip command original", &mut grant)?;
    let original = record.record().clone();
    let mut changed = original.command.clone();
    changed.rationale = text("Different complete reviewed rationale with valid new digests")?;
    let changed = seal(changed)?;
    let raw = encode_reviewed_reference(&changed, MAXIMUM)?;
    let before = grant.usage();
    invalid(
        record.decode_roundtrip_with_grant(&raw, MAXIMUM, &mut grant),
        "checked record roundtrip differs",
    )?;
    check!(eq; grant.usage().traversal_steps(), before.traversal_steps() + 3);
    check!(eq; grant.usage().encoded_bytes(), before.encoded_bytes());
    check!(eq; record.record(), &original);
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn validly_rehashed_timestamp_cannot_inherit_roundtrip_proof() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let record = draft("roundtrip timestamp original", &mut grant)?;
    let mut changed = record.record().clone();
    changed.committed_at = stamp()?
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("timestamp overflow")?;
    changed.record_digest = changed.calculated_record_digest()?;
    let raw = encode_reviewed_reference(&changed, MAXIMUM)?;
    invalid(
        record.decode_roundtrip_with_grant(&raw, MAXIMUM, &mut grant),
        "checked record roundtrip differs",
    )?;
    check!(ne; record.record().committed_at, changed.committed_at);
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn each_digest_field_difference_refuses_at_complete_roundtrip_equality() -> TestResult {
    for field in 0..3 {
        let mut grant = AttachmentReplayGrantV1::new();
        let record = draft("roundtrip digest equality", &mut grant)?;
        let mut changed = record.record().clone();
        match field {
            0 => changed.prepared_digest = digest("changed prepared")?,
            1 => changed.approval_digest = digest("changed approval")?,
            2 => changed.record_digest = digest("changed record")?,
            _ => return Err("bounded digest selector differs".into()),
        }
        let raw = serde_json::to_vec(&changed)?;
        invalid(
            record.decode_roundtrip_with_grant(&raw, MAXIMUM, &mut grant),
            "checked record roundtrip differs",
        )?;
        poisoned(&grant)?;
    }
    Ok(())
}

#[test]
fn malformed_roundtrip_and_foreign_origin_keep_distinct_primary_errors() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let record = draft("roundtrip malformed", &mut grant)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    invalid(
        record.decode_roundtrip_with_grant(b"{", MAXIMUM, &mut foreign),
        "checked command grant differs",
    )?;
    poisoned(&foreign)?;
    check!(matches!(
        record.decode_roundtrip_with_grant(b"{", MAXIMUM, &mut grant),
        Err(EvidenceSubjectAttachmentError::Decoding(_))
    ));
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn checked_commit_time_transition_retains_digest_and_review_time_rules() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let mut record = draft("checked commit transition", &mut grant)?;
    let original_command = record.record().command.clone();
    let later = stamp()?
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("timestamp overflow")?;
    let before = grant.usage().traversal_steps();
    record.set_committed_at_with_grant(later, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), before + 4);
    check!(eq; record.record().command, original_command);
    check!(eq; record.record().record_digest, record.record().calculated_record_digest()?);
    let before = record.record().clone();
    let earlier = stamp()?
        .checked_sub_signed(chrono::Duration::seconds(1))
        .ok_or("timestamp underflow")?;
    invalid(
        record.set_committed_at_with_grant(earlier, &mut grant),
        "commit precedes explicit review",
    )?;
    check!(eq; record.record(), &before);
    poisoned(&grant)?;
    Ok(())
}

#[test]
fn exhausted_commit_digest_cannot_be_encoded_using_a_fresh_grant() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let mut record = draft("commit digest exhaustion", &mut grant)?;
    let original = record.record().clone();
    let later = stamp()?
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("timestamp overflow")?;
    leave_steps(&mut grant, 2)?;
    capacity(record.set_committed_at_with_grant(later, &mut grant))?;
    poisoned(&grant)?;
    check!(eq; record.record().command, original.command);
    let mut fresh = AttachmentReplayGrantV1::new();
    invalid(
        record.encode_with_grant(MAXIMUM, &mut fresh),
        "checked command grant differs",
    )?;
    poisoned(&fresh)?;
    Ok(())
}
