use super::*;
use crate::participation_evidence::roster_source_issuance::source::tests::integration::setup::{
    enrolled_store, install,
};
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;
use census_store::{EvidenceSubjectAttachmentStoreError, RosterIssuanceInspectionRequestV1, Store};

fn repeated_team_versions(
    store: &Store,
) -> TestResult<(
    RosterIssuanceInspectionRequestV1,
    CanonicalSchool,
    CanonicalTeam,
)> {
    let (_, school, program, request) = install(store)?;
    store.append_many(Table::Teams, &[program.clone(), program.clone()])?;
    Ok((request, school, program))
}

fn parent_bytes(
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<usize> {
    evidence
        .physical_projection(grant)?
        .iter()
        .filter(|row| matches!(row.table(), Table::Schools | Table::Teams))
        .try_fold(0_usize, |sum, row| {
            sum.checked_add(row.bytes().len())
                .ok_or("parent byte overflow".into())
        })
}

#[test]
fn repeated_parent_merge_admits_exact_decode_bytes_without_store_effects() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (request, expected_school, expected_program) = repeated_team_versions(&store)?;
    let sequence = store.snapshot().sequence();
    let receipts = store.receipt_count()?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let raw_bytes = parent_bytes(evidence, grant)?;
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|remaining| remaining.checked_sub(raw_bytes))
        .ok_or("parent grant padding overflow")?;
    grant.charge_bytes(padding)?;
    let (school, program) = RosterExistingBindingAssessmentV1::read_inspected_parents(
        evidence,
        evidence.requested_school(),
        evidence.requested_program(),
        grant,
    )?;
    check!(eq; &school.id, evidence.requested_school());
    check!(eq; &program.id, evidence.requested_program());
    check!(eq; program.school, school.id);
    check!(eq; school, expected_school);
    check!(eq; program, expected_program);
    check!(eq; grant.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    grant.ensure_available()?;
    check!(RosterExistingBindingAssessmentV1::read_inspected_parents(
        evidence,
        evidence.requested_school(),
        evidence.requested_program(),
        grant,
    )
    .is_err());
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

#[test]
fn one_byte_short_parent_decode_refuses_and_poisons_without_store_effects() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (request, _, _) = repeated_team_versions(&store)?;
    let sequence = store.snapshot().sequence();
    let receipts = store.receipt_count()?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let raw_bytes = parent_bytes(evidence, grant)?;
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|remaining| remaining.checked_sub(raw_bytes))
        .and_then(|remaining| remaining.checked_add(1))
        .ok_or("short parent grant padding overflow")?;
    grant.charge_bytes(padding)?;
    check!(matches!(
        RosterExistingBindingAssessmentV1::read_inspected_parents(
            evidence, evidence.requested_school(), evidence.requested_program(), grant,
        ),
        Err(crate::CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes", needed,
                available: AttachmentReplayGrantV1::MAX_ENCODED_BYTES,
            })
        ))) if needed.checked_sub(AttachmentReplayGrantV1::MAX_ENCODED_BYTES) == Some(1)
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

#[test]
fn borrowed_inspection_keeps_bytes_and_charges_steps_but_output_still_refuses() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    let before = grant.usage().traversal_steps();
    for _ in 0..3 {
        budget::inspect_borrowed(&"escaped\n雪", &mut grant)?;
    }
    check!(eq; grant.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    check!(eq; grant.usage().traversal_steps(), before.checked_add(3).ok_or("step overflow")?);
    grant.ensure_available()?;
    check!(budget::charge(&"escaped\n雪", &mut grant).is_err());
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn borrowed_escaped_object_over_limit_refuses_and_poisons() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let limit = u64::try_from(crate::participation_evidence::MAX_PACKET)?;
    let value = "\n".repeat(crate::participation_evidence::MAX_PACKET / 2);
    check!(matches!(
        budget::inspect_borrowed(&value, &mut grant),
        Err(StoreError::BudgetExceeded {
            what: "roster source serialization measurement",
            needed,
            available,
        }) if needed > limit && available == limit
    ));
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), 1);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn borrowed_inspection_exhausted_steps_refuses_before_serialization() -> TestResult {
    struct Unserializable;
    impl serde::Serialize for Unserializable {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(
                "must refuse before serialization",
            ))
        }
    }
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
    check!(matches!(
        budget::inspect_borrowed(&Unserializable, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(EvidenceSubjectAttachmentStoreError::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps", needed,
                available: AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS,
            }
        ))) if needed.checked_sub(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS) == Some(1)
    ));
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn actual_parent_merge_over_object_limit_refuses_without_store_effects() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (_, school, _, request) = install(&store)?;
    for index in 0..70 {
        let mut version = school.clone();
        version
            .aliases
            .push(format!("{index}{}", "x".repeat(60_000)));
        check!(serde_json::to_vec(&version)?.len() < 64 * 1024);
        store.append(Table::Schools, &version)?;
    }
    let sequence = store.snapshot().sequence();
    let receipts = store.receipt_count()?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let limit = u64::try_from(crate::participation_evidence::MAX_PACKET)?;
    check!(matches!(
        RosterExistingBindingAssessmentV1::read_inspected_parents(
            evidence, evidence.requested_school(), evidence.requested_program(), grant,
        ),
        Err(crate::CrawlError::Store(StoreError::BudgetExceeded {
            what: "roster source serialization measurement", needed, available,
        })) if needed > limit && available == limit
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

#[test]
fn borrowed_object_exact_size_admits_and_one_more_encoded_byte_poisons() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let length = crate::participation_evidence::MAX_PACKET
        .checked_sub(2)
        .ok_or("object limit")?;
    let mut value = "x".repeat(length);
    budget::inspect_borrowed(&value, &mut grant)?;
    grant.ensure_available()?;
    value.push('x');
    let limit = u64::try_from(crate::participation_evidence::MAX_PACKET)?;
    check!(matches!(
        budget::inspect_borrowed(&value, &mut grant),
        Err(StoreError::BudgetExceeded {
            what: "roster source serialization measurement", needed, available,
        }) if needed.checked_sub(limit) == Some(1) && available == limit
    ));
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
