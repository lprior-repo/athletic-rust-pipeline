use super::*;

fn measure_query_and_consume_steps(case: &Committed) -> TestResult<usize> {
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("measure subject absent")?;
    let before = grant.usage().traversal_steps();
    bound.require_original_grant(&mut grant)?;
    let after = grant.usage().traversal_steps();
    Ok(after - before)
}

#[test]
fn current_capability_consumer_one_step_short_refuses_on_the_original_grant() -> TestResult {
    let case = committed("current-consumer-step-boundary")?;
    let consume_steps = measure_query_and_consume_steps(&case)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("active subject absent")?;
    let padding = MAX_STEPS
        .checked_sub(grant.usage().traversal_steps())
        .and_then(|value| value.checked_sub(consume_steps))
        .and_then(|value| value.checked_add(1))
        .ok_or("consumer work padding underflow")?;
    grant.charge_steps(padding)?;
    let before = retained_state(&case.fixture.store)?;
    check!(matches!(
        bound.require_original_grant(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: n,
                available
            }
        ))) if available == MAX_STEPS && n.checked_sub(available) == Some(1)
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}
