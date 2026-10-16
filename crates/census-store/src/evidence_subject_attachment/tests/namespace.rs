use super::*;
use crate::StoreError;
use census_domain::evidence_subject_attachment::AttachmentTextV1;

#[test]
fn one_predicate_protects_the_exact_phase_and_embedded_separator_paths() -> TestResult {
    for phase in [
        PHASE,
        "evidence_subject\0attachment/v1/record/00000000000000000001",
    ] {
        check!(matches!(
            refuse_generic_phase(phase),
            Err(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::ReservedWrite
            ))
        ));
    }
    for phase in [
        "evidence_subject_other",
        "other_evidence_subject",
        "unrelated",
    ] {
        refuse_generic_phase(phase)?;
    }
    Ok(())
}

#[test]
fn receipt_protection_operates_on_the_complete_raw_prefix() -> TestResult {
    for operation in [
        OPERATION_PREFIX,
        "evidence_subject_attachment_v1:explicit operation",
    ] {
        check!(is_reserved_operation(operation.as_bytes()));
        check!(matches!(
            refuse_generic_operation(operation),
            Err(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::ReservedWrite
            ))
        ));
    }
    let mut invalid_text = OPERATION_PREFIX.as_bytes().to_vec();
    invalid_text.push(255);
    check!(is_reserved_operation(&invalid_text));
    for operation in [
        "evidence_subject_attachment_v1",
        "evidence_subject_attachment_v10:operation",
        "other",
    ] {
        check!(!is_reserved_operation(operation.as_bytes()));
        refuse_generic_operation(operation)?;
    }
    Ok(())
}

#[test]
fn sequenced_record_keys_are_canonical_bounded_and_version_specific() -> TestResult {
    for ordinal in [1, 4_096] {
        let key = keys::record(ordinal)?;
        check!(key.starts_with(LOGICAL_PREFIX));
        check!(eq; keys::ordinal(&key)?, ordinal);
        check!(eq; key.len(), RECORD_PREFIX.len() + 20);
    }
    for key in [
        "attachment/v2/record/00000000000000000001",
        "attachment/v1/record/1",
        "attachment/v1/record/00000000000000000000",
        "attachment/v1/record/00000000000000004097",
        "attachment/v1/record/99999999999999999999",
    ] {
        check!(keys::ordinal(key).is_err());
    }
    Ok(())
}

#[test]
fn prefixed_receipt_operation_keeps_the_existing_total_byte_ceiling() -> TestResult {
    let allowed = MAX_OPERATION_BYTES - OPERATION_PREFIX.len();
    let exact = AttachmentTextV1::parse(&"x".repeat(allowed))?;
    check!(eq; keys::receipt(&exact)?.len(), MAX_OPERATION_BYTES);
    let one_over = AttachmentTextV1::parse(&"x".repeat(allowed + 1))?;
    check!(matches!(
        keys::receipt(&one_over),
        Err(StoreError::RowTooLarge {
            bytes: 513,
            max: 512
        })
    ));
    Ok(())
}
