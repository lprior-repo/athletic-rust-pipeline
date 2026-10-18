use super::{fixture::*, *};

#[test]
fn aggregate_text_and_traversal_grants_do_not_reset_between_records() -> TestResult {
    let mut bytes = validate::Grant::new();
    bytes.bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    check!(matches!(
        bytes.bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    let mut steps = validate::Grant::new();
    steps.steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
    check!(matches!(
        steps.steps(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(1)
    ));
    let record = seal(command("charge")?)?;
    let mut grant = validate::Grant::new();
    let size = codec::measure(&record, ReplayLimits::SMALL.total_text_bytes)?;
    grant.bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES - size)?;
    validate::record(&record, &mut grant)?;
    check!(matches!(
        validate::record(&record, &mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            ..
        })
    ));
    Ok(())
}

#[test]
fn one_over_records_refuses_before_record_or_receipt_validation() -> TestResult {
    let record = seal(command("same operation repeated")?)?;
    let records = vec![record; ReplayLimits::SMALL.records + 1];
    check!(matches!(
        replay_reviewed_references(&store()?, &records, &[], &AttachmentInventoryV1::empty()?),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "records",
            needed: 4_097,
            available: 4_096
        })
    ));
    Ok(())
}

#[test]
fn decoder_and_encoder_refuse_exact_one_over_byte_grants() -> TestResult {
    let record = seal(command("bounded")?)?;
    let bytes = encode_reviewed_reference(&record, ReplayLimits::SMALL.total_text_bytes)?;
    check!(matches!(
        decode_reviewed_reference(&bytes, bytes.len() - 1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "record bytes",
            ..
        })
    ));
    check!(matches!(
        encode_reviewed_reference(&record, bytes.len() - 1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "encoded bytes",
            ..
        })
    ));
    check!(matches!(
        decode_reviewed_reference(&bytes, ReplayLimits::SMALL.total_text_bytes + 1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "record byte grant",
            ..
        })
    ));
    Ok(())
}

#[test]
fn bounded_text_and_reference_decoders_reject_one_over_before_collection_growth() -> TestResult {
    check!(AttachmentTextV1::parse(&"x".repeat(ReplayLimits::SMALL.text_bytes)).is_ok());
    check!(AttachmentTextV1::parse(&"x".repeat(ReplayLimits::SMALL.text_bytes + 1)).is_err());
    let sixty_four = vec![text("bounded")?; 64];
    let bounded = list(sixty_four.clone())?;
    check!(eq;
        serde_json::from_slice::<AttachmentListV1<AttachmentTextV1>>(&serde_json::to_vec(
            &bounded
        )?)?,
        bounded
    );
    let mut too_many = sixty_four;
    too_many.push(text("one over")?);
    check!(
        serde_json::from_slice::<AttachmentListV1<AttachmentTextV1>>(&serde_json::to_vec(
            &too_many
        )?)
        .is_err()
    );
    Ok(())
}

#[test]
fn valid_projection_prefixes_share_one_traversal_budget_and_never_return_a_partial_projection(
) -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let previous = records.last().ok_or("prior record")?;
    let command = next(previous, "replacement")?;
    append(&mut records, command)?;
    let receipts = receipts(&records);
    let expected = inventory(&records)?;

    let required_steps = measure_replay_traversal_steps(&store()?, &records, &receipts, &expected)?;

    let short_precharge = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
        .checked_sub(required_steps)
        .and_then(|n| n.checked_add(1))
        .ok_or("short precharge overflow")?;
    let mut short = AttachmentReplayGrantV1::new();
    short.charge_steps(short_precharge)?;
    check!(matches!(
        replay_reviewed_references_with_grant(
            &store()?,
            &records,
            &receipts,
            &expected,
            &mut short,
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed,
            available,
        }) if needed > available
            && available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
    ));
    check!(eq; short.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    check!(matches!(
        replay_reviewed_references_with_grant(
            &store()?,
            &records,
            &receipts,
            &expected,
            &mut short,
        ),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));

    let exact_precharge = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
        .checked_sub(required_steps)
        .ok_or("exact precharge underflow")?;
    let mut exact = AttachmentReplayGrantV1::new();
    exact.charge_steps(exact_precharge)?;
    let projection = replay_reviewed_references_with_grant(
        &store()?,
        &records,
        &receipts,
        &expected,
        &mut exact,
    )?;
    check!(eq; projection.records().len(), records.len());
    check!(eq; exact.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    Ok(())
}

fn measure_replay_traversal_steps(
    store: &StoreIdentity,
    records: &[ReviewedReferenceRecordV1],
    receipts: &[AttachmentReceiptBindingV1],
    expected: &AttachmentInventoryV1,
) -> TestResult<usize> {
    let mut grant = AttachmentReplayGrantV1::new();
    match replay_reviewed_references_with_grant(store, records, receipts, expected, &mut grant) {
        Ok(projection) => {
            check!(eq; projection.records().len(), records.len());
            Ok(grant.usage().traversal_steps())
        }
        Err(error) => Err(error.into()),
    }
}

#[test]
fn bulk_decoder_admits_total_raw_bytes_before_any_record_parse() -> TestResult {
    let record = seal(command("first valid record")?)?;
    let encoded = encode_reviewed_reference(&record, ReplayLimits::SMALL.total_text_bytes)?;
    let raw_size = encoded.len();
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(raw_size)
        .and_then(|n| n.checked_add(1))
        .ok_or("byte padding overflow")?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(padding)?;
    let zero = vec![0u8; raw_size];
    check!(matches!(
        decode_and_replay_reviewed_references_with_grant(
            &store()?,
            &[zero.as_slice()],
            &[AttachmentReceiptBindingV1 {
                operation: record.command.operation.clone(),
                record_digest: record.record_digest.clone(),
            }],
            &AttachmentInventoryV1::empty()?,
            ReplayLimits::SMALL.total_text_bytes,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq; grant.usage().encoded_bytes(), padding);
    Ok(())
}

#[test]
fn bulk_decoder_charges_the_provisional_record_before_parsing_a_later_record() -> TestResult {
    let record = seal(command("first valid record")?)?;
    let encoded = encode_reviewed_reference(&record, ReplayLimits::SMALL.total_text_bytes)?;
    let raw_size = encoded.len();
    let second_raw = b"{";
    let precharge = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(raw_size)
        .and_then(|n| n.checked_sub(second_raw.len()))
        .ok_or("remaining bytes underflow")?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(precharge)?;
    check!(matches!(
        decode_and_replay_reviewed_references_with_grant(
            &store()?,
            &[encoded.as_slice(), second_raw],
            &[
                AttachmentReceiptBindingV1 {
                    operation: record.command.operation.clone(),
                    record_digest: record.record_digest.clone(),
                },
                AttachmentReceiptBindingV1 {
                    operation: text("unused")?,
                    record_digest: digest("unused")?,
                }
            ],
            &AttachmentInventoryV1::empty()?,
            ReplayLimits::SMALL.total_text_bytes,
            &mut grant
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available,
        }) if needed > available
            && available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
    ));
    check!(eq; grant.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    Ok(())
}
