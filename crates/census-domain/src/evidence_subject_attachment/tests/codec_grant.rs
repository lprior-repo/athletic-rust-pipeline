use super::{fixture::*, *};

const COMMAND_WORK: usize = 107;
const RECORD_WORK: usize = 115;
const RECORD_ENCODE_WORK: usize = 117;
const RECORD_DECODE_WORK: usize = 116;
const BORROWED_REPLAY_WORK: usize = 122;
const OWNED_REPLAY_WORK: usize = 124;

#[test]
fn fixed_fixture_validation_and_codec_pass_costs_match_the_independent_inventory() -> TestResult {
    let record = seal(command("fixed pass inventory")?)?;
    let maximum = ReplayLimits::SMALL.total_text_bytes;
    let mut command_grant = AttachmentReplayGrantV1::new();
    validate::command(&record.command, &mut command_grant)?;
    check!(eq; command_grant.usage().traversal_steps(), COMMAND_WORK);
    check!(eq; command_grant.usage().encoded_bytes(), 0);
    let mut record_grant = AttachmentReplayGrantV1::new();
    validate::record(&record, &mut record_grant)?;
    check!(eq; record_grant.usage().traversal_steps(), RECORD_WORK);
    let mut encoded_grant = AttachmentReplayGrantV1::new();
    let raw = encode_reviewed_reference_with_grant(&record, maximum, &mut encoded_grant)?;
    check!(eq; encoded_grant.usage().traversal_steps(), RECORD_ENCODE_WORK);
    check!(eq; encoded_grant.usage().encoded_bytes(), raw.len());
    check!(eq; record_grant.usage().encoded_bytes(), raw.len());
    let mut decoded_grant = AttachmentReplayGrantV1::new();
    check!(eq;
        decode_reviewed_reference_with_grant(&raw, maximum, &mut decoded_grant)?,
        record
    );
    check!(eq; decoded_grant.usage().traversal_steps(), RECORD_DECODE_WORK);
    check!(eq; decoded_grant.usage().encoded_bytes(), raw.len());
    Ok(())
}

#[test]
fn all_three_digest_paths_charge_measurement_and_hash_without_recharging_material() -> TestResult {
    let record = seal(command("three digests")?)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(eq;
        record.command.prepared_digest_with_grant(&mut grant)?,
        record.prepared_digest
    );
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(eq;
        record.command.approval_digest_with_grant(&mut grant)?,
        record.approval_digest
    );
    check!(eq; grant.usage().traversal_steps(), 4);
    check!(eq;
        record.calculated_record_digest_with_grant(&mut grant)?,
        record.record_digest
    );
    check!(eq; grant.usage().traversal_steps(), 6);
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq;
        record.prepared_digest,
        AttachmentDigestV1::of(&(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1, &record.command))?
    );
    check!(eq;
        record.record_digest,
        AttachmentDigestV1::of(&(
            RECORD_SCHEMA,
            &record.command,
            &record.prepared_digest,
            &record.approval_digest,
            record.committed_at
        ))?
    );
    Ok(())
}

#[test]
fn approval_output_is_admitted_once_and_all_encoding_passes_share_the_grant() -> TestResult {
    let command = command("approval work")?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(7)?;
    grant.charge_steps(11)?;
    let raw =
        command.approval_bytes_with_grant(ReplayLimits::SMALL.total_text_bytes, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), 11 + COMMAND_WORK + 2);
    check!(eq; grant.usage().encoded_bytes(), 7 + raw.len());
    check!(eq;
        raw,
        command.approval_bytes(ReplayLimits::SMALL.total_text_bytes)?
    );
    Ok(())
}

#[test]
fn replay_costs_include_prefix_inventory_and_complete_inventory_passes() -> TestResult {
    let records = vec![seal(command("replay work inventory")?)?];
    let receipts = receipts(&records);
    let expected = inventory(&records)?;
    let raw = encode_reviewed_reference(&records[0], ReplayLimits::SMALL.total_text_bytes)?;
    let mut grant = AttachmentReplayGrantV1::new();
    replay_reviewed_references_with_grant(&store()?, &records, &receipts, &expected, &mut grant)?;
    check!(eq; grant.usage().traversal_steps(), BORROWED_REPLAY_WORK);
    let prior_bytes = grant.usage().encoded_bytes();
    let projection = decode_and_replay_reviewed_references_with_grant(
        &store()?,
        &[&raw],
        &receipts,
        &expected,
        raw.len(),
        &mut grant,
    )?;
    check!(eq;
        grant.usage().traversal_steps(),
        BORROWED_REPLAY_WORK + OWNED_REPLAY_WORK
    );
    check!(eq; grant.usage().encoded_bytes(), prior_bytes * 2 + raw.len());
    check!(eq; projection.into_records(), records);
    Ok(())
}

#[test]
fn inventory_reuse_charges_exact_reference_walk_and_two_full_passes() -> TestResult {
    let records = vec![seal(command("inventory helper")?)?];
    let mut grant = AttachmentReplayGrantV1::new();
    check!(eq;
        calculate_reviewed_reference_inventory_with_grant(&[], &mut grant)?,
        AttachmentInventoryV1::empty()?
    );
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(eq;
        calculate_reviewed_reference_inventory_with_grant(&records, &mut grant)?,
        inventory(&records)?
    );
    check!(eq; grant.usage().traversal_steps(), 5);
    check!(eq; grant.usage().encoded_bytes(), 0);
    Ok(())
}

#[test]
fn exact_encoding_work_succeeds_and_one_less_refuses_before_the_final_pass() -> TestResult {
    let record = seal(command("encoding boundary")?)?;
    let maximum = ReplayLimits::SMALL.total_text_bytes;
    let mut exact = AttachmentReplayGrantV1::new();
    exact.charge_steps(ReplayLimits::SMALL.traversal_steps - RECORD_ENCODE_WORK)?;
    check!(eq;
        encode_reviewed_reference_with_grant(&record, maximum, &mut exact)?,
        encode_reviewed_reference(&record, maximum)?
    );
    check!(eq;
        exact.usage().traversal_steps(),
        ReplayLimits::SMALL.traversal_steps
    );
    let mut short = AttachmentReplayGrantV1::new();
    short.charge_steps(ReplayLimits::SMALL.traversal_steps - RECORD_ENCODE_WORK + 1)?;
    check!(matches!(
        encode_reviewed_reference_with_grant(&record, maximum, &mut short),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8_193,
            available: 8_192
        })
    ));
    check!(matches!(
        short.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn exhausted_work_refuses_before_decode_and_preserves_primary_capacity() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    check!(matches!(
        decode_reviewed_reference_with_grant(b"x", 1, &mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8_193,
            available: 8_192
        })
    ));
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(matches!(
        decode_reviewed_reference_with_grant(b"x", 1, &mut grant),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn validation_and_maximum_failures_poison_the_callers_original_grant() -> TestResult {
    let mut record = seal(command("invalid record")?)?;
    record.approval_digest = digest("changed")?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        encode_reviewed_reference_with_grant(
            &record,
            ReplayLimits::SMALL.total_text_bytes,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Invalid(
            "reviewed record digest or time"
        ))
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        record
            .command
            .approval_bytes_with_grant(ReplayLimits::SMALL.total_text_bytes + 1, &mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "approval byte grant",
            ..
        })
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
