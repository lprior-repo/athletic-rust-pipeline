use super::*;

mod consumer;

const MAX_STEPS: usize = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;
const MAX_BYTES: usize = AttachmentReplayGrantV1::MAX_ENCODED_BYTES;

fn measure_query_steps(case: &Committed) -> TestResult<usize> {
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let before = grant.usage().traversal_steps();
    owner.bind_exact_subject(&case.context.subject, &mut grant)?;
    let after = grant.usage().traversal_steps();
    Ok(after - before)
}

fn measure_consume_steps(case: &Committed) -> TestResult<usize> {
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

fn step_boundary(consumes: bool) -> TestResult {
    let case = committed("current-query-step-boundary")?;
    let before = retained_state(&case.fixture.store)?;
    let query_steps = measure_query_steps(&case)?;
    let consume_steps = measure_consume_steps(&case)?;
    let needed = if consumes {
        query_steps + consume_steps
    } else {
        query_steps
    };
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let padding = MAX_STEPS
        .checked_sub(grant.usage().traversal_steps())
        .and_then(|value| value.checked_sub(needed))
        .ok_or("current work padding underflow")?;
    grant.charge_steps(padding)?;
    let bound = owner.bind_exact_subject(&case.context.subject, &mut grant)?;
    let bound = bound.ok_or("exact-boundary subject absent")?;
    if consumes {
        bound.require_original_grant(&mut grant)?;
    }
    check!(eq; grant.usage().traversal_steps(), MAX_STEPS);
    check!(eq; bound.record(), &case.record);
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

fn one_step_short(consumes: bool) -> TestResult {
    let case = committed("current-query-one-step-short")?;
    let query_steps = measure_query_steps(&case)?;
    let consume_steps = measure_consume_steps(&case)?;
    let needed = if consumes {
        query_steps + consume_steps
    } else {
        query_steps
    };
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let padding = MAX_STEPS
        .checked_sub(grant.usage().traversal_steps())
        .and_then(|value| value.checked_sub(needed))
        .and_then(|value| value.checked_add(1))
        .ok_or("one-step-short padding underflow")?;
    grant.charge_steps(padding)?;
    let bound = owner.bind_exact_subject(&case.context.subject, &mut grant);
    check!(matches!(
        bound,
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: n,
                available
            }
        ))) if available == MAX_STEPS && n.checked_sub(available) == Some(1)
    ));
    poisoned(&mut grant)?;
    Ok(())
}

#[test]
fn current_query_exact_work_limit_issues_only_the_verified_borrowed_capability() -> TestResult {
    step_boundary(false)
}

#[test]
fn current_query_one_step_short_refuses_at_capability_issuance_and_poisons() -> TestResult {
    one_step_short(false)
}

#[test]
fn current_query_and_original_grant_consumption_share_the_exact_work_limit() -> TestResult {
    step_boundary(true)
}

fn byte_boundary(one_short: bool) -> TestResult {
    let case = committed("current-query-byte-boundary")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let needed = binding_bytes(&case)?
        .checked_add(case.group_bytes)
        .ok_or("query byte sum overflow")?;
    let remaining = if one_short {
        needed.checked_sub(1).ok_or("query byte underflow")?
    } else {
        needed
    };
    let padding = MAX_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|value| value.checked_sub(remaining))
        .ok_or("current byte padding underflow")?;
    grant.charge_bytes(padding)?;
    let bound = owner.bind_exact_subject(&case.context.subject, &mut grant);
    if one_short {
        check!(matches!(
            bound,
            Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
                EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate encoded bytes",
                    needed: n,
                    available
                }
            ))) if available == MAX_BYTES && n.checked_sub(available) == Some(1)
        ));
        poisoned(&mut grant)?;
    } else {
        let bound = bound?.ok_or("exact byte-boundary subject absent")?;
        check!(eq; grant.usage().encoded_bytes(), MAX_BYTES);
        check!(eq; bound.record(), &case.record);
    }
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn current_query_exact_byte_limit_admits_the_complete_target_group() -> TestResult {
    byte_boundary(false)
}

#[test]
fn current_query_one_byte_short_refuses_the_target_group_before_issuance() -> TestResult {
    byte_boundary(true)
}
