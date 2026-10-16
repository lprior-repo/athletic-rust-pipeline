use super::*;
use crate::participation_candidates::MAX_PARTICIPATION_FACTS;

pub(super) use super::AttachmentReplayGrantV1 as Grant;

mod lookup;

pub(super) fn record(record: &ReviewedReferenceRecordV1, grant: &mut Grant) -> Result<()> {
    record_prefix(record, grant)?;
    command(&record.command, grant)?;
    record_suffix(record, grant)
}

pub(super) fn record_shell(record: &ReviewedReferenceRecordV1, grant: &mut Grant) -> Result<()> {
    record_prefix(record, grant)?;
    record_suffix(record, grant)
}

fn record_prefix(record: &ReviewedReferenceRecordV1, grant: &mut Grant) -> Result<()> {
    grant.steps(1)?;
    let bytes = codec::measure_with_grant(record, ReplayLimits::SMALL.total_text_bytes, grant)?;
    grant.bytes(bytes)
}

fn record_suffix(record: &ReviewedReferenceRecordV1, grant: &mut Grant) -> Result<()> {
    if record.prepared_digest != record.command.prepared_digest_with_grant(grant)?
        || record.approval_digest != record.command.approval_digest_with_grant(grant)?
        || record.record_digest != record.calculated_record_digest_with_grant(grant)?
        || record.committed_at < record.command.reviewed_at
    {
        return Err(EvidenceSubjectAttachmentError::Invalid(
            "reviewed record digest or time",
        ));
    }
    Ok(())
}

pub(super) fn command(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut Grant,
) -> Result<()> {
    grant.steps(1)?;
    if command.version != 1
        || command.policy.as_str() != EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1
        || command.expected_attachment_inventory.version != 1
        || command.subjects.is_empty()
        || !command.target.canonical_candidate.validate("ath")
    {
        return Err(EvidenceSubjectAttachmentError::Invalid("command schema"));
    }
    admit_context(command, grant)?;
    references(command, grant)?;
    corroboration(command, grant)
}

fn admit_context(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut Grant,
) -> Result<()> {
    let context = &command.reviewed_context;
    let count = [
        command.subjects.len(),
        context.objects.len(),
        context.facts.len(),
        context.corroboration.len(),
        context.complete_scope_records.len(),
        context.accepted_target_subjects.len(),
    ]
    .into_iter()
    .try_fold(0usize, |sum, value| {
        sum.checked_add(value)
            .ok_or(EvidenceSubjectAttachmentError::Overflow)
    })?;
    limit("combined references", count, MAX_PARTICIPATION_FACTS)?;
    grant.steps(count)?;
    unique(
        command
            .subjects
            .as_slice()
            .iter()
            .map(|subject| &subject.subject),
        grant,
    )?;
    unique(
        context
            .objects
            .as_slice()
            .iter()
            .map(|object| &object.digest),
        grant,
    )?;
    unique(
        context.facts.as_slice().iter().map(|fact| &fact.fact),
        grant,
    )?;
    unique(context.complete_scope_records.as_slice().iter(), grant)?;
    unique(context.accepted_target_subjects.as_slice().iter(), grant)?;
    for object in context.objects.as_slice() {
        grant.steps(1)?;
        if object.bytes == 0 {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "empty retained object",
            ));
        }
    }
    if context.objects.is_empty() || context.complete_scope_records.is_empty() {
        return Err(EvidenceSubjectAttachmentError::Invalid(
            "missing retained context inventory",
        ));
    }
    Ok(())
}

fn references(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut Grant,
) -> Result<()> {
    let context = &command.reviewed_context;
    object(context, &context.accepted_target_witness, grant)?;
    for subject in command.subjects.as_slice() {
        for reference in [
            &subject.capture,
            &subject.body,
            &subject.interpretation,
            &subject.locator,
            &subject.subject,
            &subject.context,
        ] {
            object(context, reference, grant)?;
        }
    }
    for subject in context.accepted_target_subjects.as_slice() {
        object(context, subject, grant)?;
    }
    for scope in context.complete_scope_records.as_slice() {
        object(context, scope, grant)?;
    }
    for fact in context.facts.as_slice() {
        for reference in [&fact.fact, &fact.subject, &fact.publisher_lineage] {
            object(context, reference, grant)?;
        }
    }
    Ok(())
}

fn unique<'a>(
    values: impl Iterator<Item = &'a AttachmentDigestV1>,
    grant: &mut Grant,
) -> Result<()> {
    let mut previous = None;
    for value in values {
        grant.steps(1)?;
        if previous.is_some_and(|prior| prior >= value) {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "unordered or duplicate references",
            ));
        }
        previous = Some(value);
    }
    Ok(())
}

fn object(
    context: &ReviewedAttachmentContextV1,
    reference: &AttachmentDigestV1,
    grant: &mut Grant,
) -> Result<()> {
    let object = lookup::find(
        context.objects.as_slice(),
        reference,
        |object| &object.digest,
        grant,
    )?
    .ok_or(EvidenceSubjectAttachmentError::Invalid(
        "missing retained object reference",
    ))?;
    if object.bytes == 0 {
        return Err(EvidenceSubjectAttachmentError::Invalid(
            "empty retained object",
        ));
    }
    Ok(())
}

fn fact<'a>(
    context: &'a ReviewedAttachmentContextV1,
    reference: &AttachmentDigestV1,
    grant: &mut Grant,
) -> Result<&'a ReviewedAttachmentFactV1> {
    lookup::find(
        context.facts.as_slice(),
        reference,
        |fact| &fact.fact,
        grant,
    )?
    .ok_or(EvidenceSubjectAttachmentError::Invalid(
        "missing corroborating fact",
    ))
}

fn corroboration(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut Grant,
) -> Result<()> {
    let context = &command.reviewed_context;
    if matches!(command.action, AttachmentActionV1::Attach { .. })
        && context.corroboration.is_empty()
    {
        return Err(EvidenceSubjectAttachmentError::Invalid(
            "missing paired corroboration",
        ));
    }
    for pair in context.corroboration.as_slice() {
        let subject = fact(context, &pair.subject_fact, grant)?;
        let target = fact(context, &pair.accepted_target_fact, grant)?;
        if subject.fact == target.fact
            || subject.subject == target.subject
            || !contains_subject(command, &subject.subject, grant)?
            || !contains_target(context, &target.subject, grant)?
            || subject.disposition != AttachmentFactDispositionV1::SupportsAttachment
            || target.disposition != AttachmentFactDispositionV1::SupportsAttachment
        {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "corroboration pair references",
            ));
        }
        if pair.basis == AttachmentCorroborationBasisV1::IndependentPersonSpecificContext
            && subject.publisher_lineage == target.publisher_lineage
        {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "same-lineage corroboration",
            ));
        }
    }
    if matches!(command.action, AttachmentActionV1::Attach { .. }) {
        complete_pairs(command, grant)?;
    }
    Ok(())
}

fn complete_pairs(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut Grant,
) -> Result<()> {
    let context = &command.reviewed_context;
    for subject in command.subjects.as_slice() {
        let mut found = false;
        for pair in context.corroboration.as_slice() {
            grant.steps(1)?;
            if fact(context, &pair.subject_fact, grant)?.subject == subject.subject {
                found = true;
                break;
            }
        }
        if !found {
            return Err(EvidenceSubjectAttachmentError::Invalid(
                "unpaired attached subject",
            ));
        }
    }
    Ok(())
}

fn contains_subject(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    subject: &AttachmentDigestV1,
    grant: &mut Grant,
) -> Result<bool> {
    for candidate in command.subjects.as_slice() {
        grant.steps(1)?;
        if &candidate.subject == subject {
            return Ok(true);
        }
    }
    Ok(false)
}

fn contains_target(
    context: &ReviewedAttachmentContextV1,
    subject: &AttachmentDigestV1,
    grant: &mut Grant,
) -> Result<bool> {
    for candidate in context.accepted_target_subjects.as_slice() {
        grant.steps(1)?;
        if candidate == subject {
            return Ok(true);
        }
    }
    Ok(false)
}
