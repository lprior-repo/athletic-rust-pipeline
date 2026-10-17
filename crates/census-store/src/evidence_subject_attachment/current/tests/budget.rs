use super::*;

mod consumer;

const QUERY_STEPS: usize = 30;
const CONSUME_STEPS: usize = 17;
const MAX_BYTES: usize = 4 * 1024 * 1024;

#[test]
fn current_reader_adds_only_two_constructor_steps_to_the_complete_historical_read() -> TestResult {
    let case = committed("current-reader-complete-load")?;
    let view = case.fixture.store.snapshot();
    let mut historical = AttachmentReplayGrantV1::new();
    view.read_evidence_subject_attachment_references_v1(&mut historical)?;
    let mut current = AttachmentReplayGrantV1::new();
    view.read_current_evidence_subject_attachment_references_v1(&mut current)?;
    check!(eq; current.usage().traversal_steps(), historical.usage().traversal_steps().checked_add(2).ok_or("constructor steps overflow")?);
    check!(eq; current.usage().encoded_bytes(), historical.usage().encoded_bytes());
    Ok(())
}

#[test]
fn repeated_current_queries_and_consumers_debit_all_named_projection_and_target_passes(
) -> TestResult {
    let case = committed("current-query-work")?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let projection_bytes = binding_bytes(&case)?;
    for _ in 0..2 {
        let steps = grant.usage().traversal_steps();
        let bytes = grant.usage().encoded_bytes();
        let bound = owner
            .bind_exact_subject(&case.context.subject, &mut grant)?
            .ok_or("active subject absent")?;
        check!(eq; grant.usage().traversal_steps(), steps.checked_add(QUERY_STEPS).ok_or("query steps overflow")?);
        check!(eq; grant.usage().encoded_bytes(), bytes.checked_add(projection_bytes).and_then(|n| n.checked_add(case.group_bytes)).ok_or("query bytes overflow")?);
        bound.require_original_grant(&mut grant)?;
        check!(eq; grant.usage().traversal_steps(), steps.checked_add(QUERY_STEPS).and_then(|n| n.checked_add(CONSUME_STEPS)).ok_or("consume steps overflow")?);
        check!(eq; grant.usage().encoded_bytes(), bytes.checked_add(projection_bytes).and_then(|n| n.checked_add(case.group_bytes)).and_then(|n| n.checked_add(projection_bytes)).ok_or("consume bytes overflow")?);
    }
    Ok(())
}

fn work_boundary(remaining: usize, consumes: bool) -> TestResult {
    let case = committed("current-query-step-boundary")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let padding = 8192usize
        .checked_sub(grant.usage().traversal_steps())
        .and_then(|value| value.checked_sub(remaining))
        .ok_or("current work padding underflow")?;
    grant.charge_steps(padding)?;
    let bound = owner.bind_exact_subject(&case.context.subject, &mut grant);
    if remaining == 29 {
        check!(matches!(
            bound,
            Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
                EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate traversal steps",
                    needed: 8193,
                    available: 8192
                }
            )))
        ));
        poisoned(&mut grant)?;
    } else {
        let bound = bound?.ok_or("exact-boundary subject absent")?;
        if consumes {
            bound.require_original_grant(&mut grant)?;
        }
        check!(eq; grant.usage().traversal_steps(), 8192);
        check!(eq; bound.record(), &case.record);
    }
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn current_query_exact_work_limit_issues_only_the_verified_borrowed_capability() -> TestResult {
    work_boundary(QUERY_STEPS, false)
}

#[test]
fn current_query_one_step_short_refuses_at_capability_issuance_and_poisons() -> TestResult {
    work_boundary(29, false)
}

#[test]
fn current_query_and_original_grant_consumption_share_the_exact_work_limit() -> TestResult {
    work_boundary(47, true)
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
                    needed: 4_194_305,
                    available: 4_194_304
                }
            )))
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
