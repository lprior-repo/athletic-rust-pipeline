use super::*;
use crate::StoreError;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError as GrantError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exhausted_encoding_preserves_primary_capacity_and_poisoning() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let limit = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
    let needed = limit.checked_add(1).ok_or("expected traversal overflow")?;
    grant.charge_steps(limit)?;
    let before = grant.usage();
    let result = encode(&true, &mut grant);
    check!(matches!(result,
        Err(StoreError::EvidenceSubjectAttachment(
            crate::EvidenceSubjectAttachmentStoreError::Domain(
                GrantError::Capacity { resource: "aggregate traversal steps", needed: found, available }
            )
        )) if found == needed && available == limit));
    check!(eq; grant.usage(), before);
    check!(matches!(
        grant.ensure_available(),
        Err(GrantError::Poisoned)
    ));
    Ok(())
}

#[test]
fn exhausted_canonical_verification_preserves_primary_capacity_and_poisoning() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let limit = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
    let needed = limit.checked_add(1).ok_or("expected traversal overflow")?;
    grant.charge_steps(limit)?;
    let before = grant.usage();
    let result = require_canonical(&true, b"true", &mut grant);
    check!(matches!(result,
        Err(StoreError::EvidenceSubjectAttachment(
            crate::EvidenceSubjectAttachmentStoreError::Domain(
                GrantError::Capacity { resource: "aggregate traversal steps", needed: found, available }
            )
        )) if found == needed && available == limit));
    check!(eq; grant.usage(), before);
    check!(matches!(
        grant.ensure_available(),
        Err(GrantError::Poisoned)
    ));
    Ok(())
}
