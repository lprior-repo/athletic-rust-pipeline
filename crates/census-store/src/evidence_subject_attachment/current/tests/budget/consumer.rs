use super::*;

#[test]
fn current_capability_consumer_one_step_short_refuses_on_the_original_grant() -> TestResult {
    let case = committed("current-consumer-step-boundary")?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("active subject absent")?;
    let padding = 8192usize
        .checked_sub(grant.usage().traversal_steps())
        .and_then(|value| value.checked_sub(16))
        .ok_or("consumer work padding underflow")?;
    grant.charge_steps(padding)?;
    let before = retained_state(&case.fixture.store)?;
    check!(matches!(
        bound.require_original_grant(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: 8193,
                available: 8192
            }
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn current_capability_consumer_one_byte_short_refuses_on_the_original_grant() -> TestResult {
    let case = committed("current-consumer-byte-boundary")?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("active subject absent")?;
    let remaining = binding_bytes(&case)?
        .checked_sub(1)
        .ok_or("consumer byte underflow")?;
    let padding = MAX_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|value| value.checked_sub(remaining))
        .ok_or("consumer byte padding underflow")?;
    grant.charge_bytes(padding)?;
    let before = retained_state(&case.fixture.store)?;
    check!(matches!(
        bound.require_original_grant(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes",
                needed: 4_194_305,
                available: 4_194_304
            }
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}
