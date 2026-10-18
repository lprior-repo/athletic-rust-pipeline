use super::{fixture::*, *};

fn refuses(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    expected: &'static str,
) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    match command.approval_bytes_with_grant(ReplayLimits::SMALL.total_text_bytes, &mut grant) {
        Err(EvidenceSubjectAttachmentError::Invalid(actual)) => check!(eq; actual, expected),
        other => return Err(format!("expected {expected:?}, got {other:?}").into()),
    }
    let usage = grant.usage();
    check!(matches!(
        command.approval_bytes_with_grant(ReplayLimits::SMALL.total_text_bytes, &mut grant),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; grant.usage(), usage);
    Ok(())
}

#[test]
fn unordered_and_duplicate_objects_refuse_before_reference_lookup() -> TestResult {
    let original = command("ordered objects")?;
    let mut reversed = original.clone();
    let mut objects = reversed.reviewed_context.objects.as_slice().to_vec();
    objects.reverse();
    reversed.reviewed_context.objects = list(objects)?;
    reversed.reviewed_context.accepted_target_witness = digest("not retained")?;
    refuses(&reversed, "unordered or duplicate references")?;
    let mut duplicate = original;
    let mut objects = duplicate.reviewed_context.objects.as_slice().to_vec();
    let first = objects.first().ok_or("first object absent")?.clone();
    objects.insert(1, AttachmentObjectReferenceV1 { bytes: 99, ..first });
    duplicate.reviewed_context.objects = list(objects)?;
    refuses(&duplicate, "unordered or duplicate references")
}

#[test]
fn unordered_and_duplicate_facts_refuse_before_pair_lookup() -> TestResult {
    let original = command("ordered facts")?;
    let mut reversed = original.clone();
    let mut facts = reversed.reviewed_context.facts.as_slice().to_vec();
    facts.reverse();
    reversed.reviewed_context.facts = list(facts)?;
    refuses(&reversed, "unordered or duplicate references")?;
    let mut duplicate = original;
    let mut facts = duplicate.reviewed_context.facts.as_slice().to_vec();
    let mut first = facts.first().ok_or("first fact absent")?.clone();
    first.disposition = AttachmentFactDispositionV1::ContradictsAttachment;
    facts.insert(1, first);
    duplicate.reviewed_context.facts = list(facts)?;
    refuses(&duplicate, "unordered or duplicate references")
}

#[test]
fn missing_object_and_pair_fact_keep_their_exact_absence_errors() -> TestResult {
    let mut absent_object = command("missing object")?;
    absent_object.reviewed_context.accepted_target_witness = digest("not retained")?;
    refuses(&absent_object, "missing retained object reference")?;
    let mut absent_fact = command("missing fact")?;
    let mut pairs = absent_fact
        .reviewed_context
        .corroboration
        .as_slice()
        .to_vec();
    pairs.first_mut().ok_or("first pair absent")?.subject_fact = digest("missing paired fact")?;
    absent_fact.reviewed_context.corroboration = list(pairs)?;
    refuses(&absent_fact, "missing corroborating fact")
}

#[test]
fn zero_object_bytes_remain_invalid_after_strict_digest_order_admission() -> TestResult {
    let mut candidate = command("zero object bytes")?;
    let mut objects = candidate.reviewed_context.objects.as_slice().to_vec();
    objects.first_mut().ok_or("first object absent")?.bytes = 0;
    candidate.reviewed_context.objects = list(objects)?;
    refuses(&candidate, "empty retained object")
}

#[test]
fn exact_prelookup_work_refuses_before_the_first_midpoint_and_poison_is_sticky() -> TestResult {
    let candidate = command("first midpoint admission")?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS - 51)?;
    check!(matches!(
        candidate.approval_bytes_with_grant(ReplayLimits::SMALL.total_text_bytes, &mut grant),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
