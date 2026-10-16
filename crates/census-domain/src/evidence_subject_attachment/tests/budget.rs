use super::{fixture::*, *};

#[test]
fn aggregate_text_and_traversal_grants_do_not_reset_between_records() -> TestResult {
    let mut bytes = validate::Grant::new();
    bytes.bytes(ReplayLimits::SMALL.total_text_bytes)?;
    check!(matches!(
        bytes.bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed: 4_194_305,
            available: 4_194_304,
        })
    ));
    let mut steps = validate::Grant::new();
    steps.steps(ReplayLimits::SMALL.traversal_steps)?;
    check!(matches!(
        steps.steps(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8_193,
            available: 8_192,
        })
    ));
    let record = seal(command("charge")?)?;
    let mut grant = validate::Grant::new();
    let size = codec::measure(&record, ReplayLimits::SMALL.total_text_bytes)?;
    grant.bytes(ReplayLimits::SMALL.total_text_bytes - size)?;
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
    let mut last_success = 0;
    for index in 1..=128 {
        let result = replay_reviewed_references(
            &store()?,
            &records,
            &receipts(&records),
            &inventory(&records)?,
        );
        match result {
            Ok(projection) => {
                check!(eq; projection.records().len(), records.len());
                check!(eq; projection.active_records().count(), 1);
                last_success = records.len();
            }
            Err(EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed,
                available,
            }) => {
                check!(last_success > 0);
                check!(eq; records.len(), last_success + 1);
                check!(needed > available);
                check!(eq; available, ReplayLimits::SMALL.traversal_steps);
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        }
        let previous = records.last().ok_or("prior record")?;
        let command = next(previous, &format!("replacement-{index}"))?;
        append(&mut records, command)?;
    }
    Err("projection traversal budget was reset or not charged".into())
}

#[test]
fn bulk_decoder_admits_total_raw_bytes_before_any_record_parse() -> TestResult {
    let bytes = vec![b'x'; ReplayLimits::SMALL.total_text_bytes];
    let raw: [&[u8]; 2] = [&bytes, b"x"];
    let receipt = AttachmentReceiptBindingV1 {
        operation: text("unused")?,
        record_digest: digest("unused")?,
    };
    check!(matches!(
        decode_and_replay_reviewed_references(
            &store()?,
            &raw,
            &[receipt.clone(), receipt],
            &AttachmentInventoryV1::empty()?,
            ReplayLimits::SMALL.total_text_bytes
        ),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed: 4_194_305,
            available: 4_194_304
        })
    ));
    Ok(())
}

#[test]
fn bulk_decoder_charges_the_provisional_record_before_parsing_a_later_record() -> TestResult {
    let record = seal(command("first valid record")?)?;
    let mut first = encode_reviewed_reference(&record, ReplayLimits::SMALL.total_text_bytes)?;
    first.resize(ReplayLimits::SMALL.total_text_bytes - 1, b' ');
    let raw: [&[u8]; 2] = [&first, b"x"];
    let receipt = AttachmentReceiptBindingV1 {
        operation: record.command.operation.clone(),
        record_digest: record.record_digest.clone(),
    };
    check!(
        matches!(decode_and_replay_reviewed_references(&store()?, &raw, &[receipt.clone(), receipt], &AttachmentInventoryV1::empty()?, ReplayLimits::SMALL.total_text_bytes),
        Err(EvidenceSubjectAttachmentError::Capacity { resource: "aggregate encoded bytes", needed, available })
            if needed > available && available == ReplayLimits::SMALL.total_text_bytes)
    );
    Ok(())
}
