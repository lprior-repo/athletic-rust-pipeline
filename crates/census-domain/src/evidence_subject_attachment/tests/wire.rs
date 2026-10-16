use super::{fixture::*, *};
use sha2::{Digest, Sha256};

#[test]
fn bounded_wire_roundtrip_preserves_exact_reviewed_references() -> TestResult {
    let record = seal(command("reviewed")?)?;
    let bytes = encode_reviewed_reference(&record, ReplayLimits::SMALL.total_text_bytes)?;
    check!(eq; decode_reviewed_reference(&bytes, bytes.len())?, record);
    let approval = record
        .command
        .approval_bytes(ReplayLimits::SMALL.total_text_bytes)?;
    let mut hash = Sha256::new();
    hash.update(APPROVAL_SCHEMA.as_bytes());
    hash.update([0]);
    hash.update(approval);
    check!(eq;
        record.approval_digest.as_str(),
        format!("{:x}", hash.finalize())
    );
    Ok(())
}

#[test]
fn changed_record_approval_and_unrecognized_fields_fail_closed() -> TestResult {
    let mut record = seal(command("reviewed")?)?;
    record.command.rationale = text("Changed review content")?;
    check!(encode_reviewed_reference(&record, ReplayLimits::SMALL.total_text_bytes).is_err());
    let record = seal(command("reviewed")?)?;
    let mut wire = serde_json::to_value(&record)?;
    wire["extra"] = serde_json::Value::Bool(true);
    check!(decode_reviewed_reference(
        &serde_json::to_vec(&wire)?,
        ReplayLimits::SMALL.total_text_bytes
    )
    .is_err());
    wire.as_object_mut().ok_or("wire object")?.remove("extra");
    wire["approval_digest"] = serde_json::Value::String("0".repeat(64));
    check!(decode_reviewed_reference(
        &serde_json::to_vec(&wire)?,
        ReplayLimits::SMALL.total_text_bytes
    )
    .is_err());
    Ok(())
}

#[test]
fn name_only_unpaired_and_same_lineage_claims_cannot_form_structural_attach() -> TestResult {
    let mut candidate = command("unpaired")?;
    candidate.reviewed_context.corroboration = list(vec![])?;
    check!(
        encode_reviewed_reference(&seal(candidate)?, ReplayLimits::SMALL.total_text_bytes).is_err()
    );
    let mut candidate = command("same lineage")?;
    let mut facts = candidate.reviewed_context.facts.as_slice().to_vec();
    facts[1].publisher_lineage = facts[0].publisher_lineage.clone();
    candidate.reviewed_context.facts = list(facts)?;
    let mut pairs = candidate.reviewed_context.corroboration.as_slice().to_vec();
    pairs[0].basis = AttachmentCorroborationBasisV1::IndependentPersonSpecificContext;
    candidate.reviewed_context.corroboration = list(pairs)?;
    check!(matches!(
        encode_reviewed_reference(&seal(candidate)?, ReplayLimits::SMALL.total_text_bytes),
        Err(EvidenceSubjectAttachmentError::Invalid(
            "same-lineage corroboration"
        ))
    ));
    Ok(())
}

#[test]
fn pair_endpoints_must_name_attached_and_retained_target_subjects() -> TestResult {
    let mut candidate = command("wrong endpoint")?;
    candidate.reviewed_context.accepted_target_subjects = list(vec![])?;
    check!(matches!(
        encode_reviewed_reference(&seal(candidate)?, ReplayLimits::SMALL.total_text_bytes),
        Err(EvidenceSubjectAttachmentError::Invalid(
            "corroboration pair references"
        ))
    ));
    let mut candidate = command("empty object")?;
    let mut objects = candidate.reviewed_context.objects.as_slice().to_vec();
    objects[0].bytes = 0;
    candidate.reviewed_context.objects = list(objects)?;
    check!(matches!(
        encode_reviewed_reference(&seal(candidate)?, ReplayLimits::SMALL.total_text_bytes),
        Err(EvidenceSubjectAttachmentError::Invalid(
            "empty retained object"
        ))
    ));
    Ok(())
}

#[test]
fn complete_owned_wire_reader_replays_the_same_structural_projection() -> TestResult {
    let mut records = Vec::new();
    append(&mut records, command("first")?)?;
    let next = next(&records[0], "next")?;
    append(&mut records, next)?;
    let encoded: Vec<Vec<u8>> = records
        .iter()
        .map(|record| encode_reviewed_reference(record, ReplayLimits::SMALL.total_text_bytes))
        .collect::<std::result::Result<_, _>>()?;
    let raw: Vec<&[u8]> = encoded.iter().map(Vec::as_slice).collect();
    let decoded = decode_and_replay_reviewed_references(
        &store()?,
        &raw,
        &receipts(&records),
        &inventory(&records)?,
        ReplayLimits::SMALL.total_text_bytes,
    )?;
    check!(eq; decoded.records(), records);
    check!(eq;
        decoded.active_records().collect::<Vec<_>>(),
        vec![&records[1]]
    );
    check!(eq; decoded.inventory(), replay(&records)?.inventory());
    Ok(())
}

#[test]
fn every_attached_subject_requires_its_own_retained_pair_endpoint() -> TestResult {
    let mut candidate = command("dangling second subject")?;
    let extra = subject("second subject")?;
    add_subject(&mut candidate, extra.clone())?;
    check!(matches!(
        encode_reviewed_reference(
            &seal(candidate.clone())?,
            ReplayLimits::SMALL.total_text_bytes
        ),
        Err(EvidenceSubjectAttachmentError::Invalid(
            "unpaired attached subject"
        ))
    ));
    pair_subject(&mut candidate, extra.subject)?;
    check!(
        encode_reviewed_reference(&seal(candidate)?, ReplayLimits::SMALL.total_text_bytes).is_ok()
    );
    Ok(())
}
