use super::{fixture::*, *};

mod limits;

#[test]
fn borrowed_replay_and_later_planning_share_the_original_byte_grant() -> TestResult {
    let records = vec![seal(command("shared borrowed")?)?];
    let receipts = receipts(&records);
    let expected = inventory(&records)?;
    let size = codec::measure(&records[0], ReplayLimits::SMALL.total_text_bytes)?
        + codec::measure(&receipts[0], ReplayLimits::SMALL.total_text_bytes)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES - size - 1)?;
    grant.charge_steps(7)?;
    let projection = replay_reviewed_references_with_grant(
        &store()?,
        &records,
        &receipts,
        &expected,
        &mut grant,
    )?;
    check!(eq; projection.records(), records.as_slice());
    check!(eq; projection.inventory(), &expected);
    check!(eq;
        grant.usage().encoded_bytes(),
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES - 1
    );
    check!(grant.usage().traversal_steps() > 7);
    grant.charge_bytes(1)?;
    check!(matches!(
        grant.charge_bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            ..
        })
    ));
    Ok(())
}

#[test]
fn owned_replay_charges_raw_and_validated_bytes_into_the_same_grant() -> TestResult {
    let records = vec![seal(command("shared owned")?)?];
    let receipts = receipts(&records);
    let expected = inventory(&records)?;
    let raw = encode_reviewed_reference(&records[0], ReplayLimits::SMALL.total_text_bytes)?;
    let size = raw.len()
        + codec::measure(&records[0], ReplayLimits::SMALL.total_text_bytes)?
        + codec::measure(&receipts[0], ReplayLimits::SMALL.total_text_bytes)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES - size)?;
    let projection = decode_and_replay_reviewed_references_with_grant(
        &store()?,
        &[&raw],
        &receipts,
        &expected,
        raw.len(),
        &mut grant,
    )?;
    check!(eq; projection.records(), records.as_slice());
    check!(eq; projection.inventory(), &expected);
    check!(eq;
        grant.usage().encoded_bytes(),
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES
    );
    check!(matches!(
        grant.charge_bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            ..
        })
    ));
    Ok(())
}

#[test]
fn failed_receipt_validation_poison_prevents_a_valid_retry_on_the_same_grant() -> TestResult {
    let records = vec![seal(command("receipt binding")?)?];
    let good = receipts(&records);
    let mut bad = good.clone();
    bad[0].record_digest = digest("wrong receipt")?;
    let expected = inventory(&records)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay_reviewed_references_with_grant(&store()?, &records, &bad, &expected, &mut grant),
        Err(EvidenceSubjectAttachmentError::Invalid(
            "record origin or receipt binding"
        ))
    ));
    let usage = grant.usage();
    check!(matches!(
        replay_reviewed_references_with_grant(&store()?, &records, &good, &expected, &mut grant),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; grant.usage(), usage);
    Ok(())
}

#[test]
fn failed_inventory_validation_keeps_the_exact_primary_conflict() -> TestResult {
    let records = vec![seal(command("inventory binding")?)?];
    let receipts = receipts(&records);
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay_reviewed_references_with_grant(
            &store()?,
            &records,
            &receipts,
            &AttachmentInventoryV1::empty()?,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Conflict(
            "complete attachment inventory"
        ))
    ));
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn decode_failure_poison_is_preserved_without_replacing_the_json_error() -> TestResult {
    let receipt = AttachmentReceiptBindingV1 {
        operation: text("unused")?,
        record_digest: digest("unused")?,
    };
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        decode_and_replay_reviewed_references_with_grant(
            &store()?,
            &[b"x"],
            &[receipt],
            &AttachmentInventoryV1::empty()?,
            1,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Decoding(_))
    ));
    let usage = grant.usage();
    check!(matches!(
        decode_and_replay_reviewed_references_with_grant(
            &store()?,
            &[],
            &[],
            &AttachmentInventoryV1::empty()?,
            1,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; grant.usage(), usage);
    Ok(())
}

#[test]
fn precharged_total_refuses_before_decoding_and_never_resets() -> TestResult {
    let receipt = AttachmentReceiptBindingV1 {
        operation: text("unused")?,
        record_digest: digest("unused")?,
    };
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    check!(matches!(
        decode_and_replay_reviewed_references_with_grant(
            &store()?,
            &[b"x"],
            &[receipt],
            &AttachmentInventoryV1::empty()?,
            1,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn external_failure_poison_blocks_even_an_empty_projection() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(9)?;
    grant.charge_steps(3)?;
    grant.poison();
    let usage = grant.usage();
    check!(matches!(
        replay_reviewed_references_with_grant(
            &store()?,
            &[],
            &[],
            &AttachmentInventoryV1::empty()?,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; grant.usage(), usage);
    Ok(())
}

#[test]
fn replay_uses_the_exact_remaining_step_budget() -> TestResult {
    let records = vec![seal(command("remaining steps")?)?];
    let receipts = receipts(&records);
    let expected = inventory(&records)?;
    let mut measured = AttachmentReplayGrantV1::new();
    replay_reviewed_references_with_grant(
        &store()?,
        &records,
        &receipts,
        &expected,
        &mut measured,
    )?;
    let steps = measured.usage().traversal_steps();
    check!(steps > 0 && steps < AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    let mut exact = AttachmentReplayGrantV1::new();
    exact.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS - steps)?;
    replay_reviewed_references_with_grant(&store()?, &records, &receipts, &expected, &mut exact)?;
    check!(eq;
        exact.usage().traversal_steps(),
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
    );
    let mut one_over = AttachmentReplayGrantV1::new();
    one_over.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS - steps + 1)?;
    check!(matches!(
        replay_reviewed_references_with_grant(
            &store()?,
            &records,
            &receipts,
            &expected,
            &mut one_over
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(1)
    ));
    check!(matches!(
        one_over.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn borrowed_and_owned_replays_preserve_one_monotonic_usage() -> TestResult {
    let records = vec![seal(command("two bounded revalidations")?)?];
    let receipts = receipts(&records);
    let expected = inventory(&records)?;
    let raw = encode_reviewed_reference(&records[0], ReplayLimits::SMALL.total_text_bytes)?;
    let mut grant = AttachmentReplayGrantV1::new();
    replay_reviewed_references_with_grant(&store()?, &records, &receipts, &expected, &mut grant)?;
    let first = grant.usage();
    decode_and_replay_reviewed_references_with_grant(
        &store()?,
        &[&raw],
        &receipts,
        &expected,
        raw.len(),
        &mut grant,
    )?;
    check!(eq;
        grant.usage().encoded_bytes(),
        first.encoded_bytes() * 2 + raw.len()
    );
    check!(eq;
        grant.usage().traversal_steps(),
        first.traversal_steps() * 2 + 2
    );
    Ok(())
}
