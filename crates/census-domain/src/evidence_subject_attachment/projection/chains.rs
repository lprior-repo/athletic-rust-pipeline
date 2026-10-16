use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum State {
    Active,
    Revoked,
    Superseded,
}

pub(super) fn accept(
    record: &ReviewedReferenceRecordV1,
    prefix: &[ReviewedReferenceRecordV1],
    states: &mut Vec<State>,
    grant: &mut validate::Grant,
) -> Result<()> {
    let latest = latest(record, prefix, grant)?;
    let next = match &record.command.action {
        AttachmentActionV1::Attach {
            prior: AttachmentPredecessorV1::First,
        } => {
            if latest.is_some() {
                return Err(EvidenceSubjectAttachmentError::Conflict(
                    "second first record",
                ));
            }
            State::Active
        }
        AttachmentActionV1::Attach {
            prior: AttachmentPredecessorV1::After(prior),
        } => {
            replace(record, prior, latest, prefix, states, grant)?;
            State::Active
        }
        AttachmentActionV1::Revoke { prior } => {
            revoke(record, prior, latest, prefix, states, grant)?;
            State::Revoked
        }
    };
    if next == State::Active {
        overlaps(record, prefix, states, grant)?;
    }
    states.push(next);
    Ok(())
}

fn replace(
    record: &ReviewedReferenceRecordV1,
    prior: &AttachmentRecordReferenceV1,
    latest: Option<usize>,
    prefix: &[ReviewedReferenceRecordV1],
    states: &mut [State],
    grant: &mut validate::Grant,
) -> Result<()> {
    let index = predecessor(record, prior, latest, prefix, grant)?;
    let state = states
        .get_mut(index)
        .ok_or(EvidenceSubjectAttachmentError::Invalid("predecessor state"))?;
    if *state == State::Active
        && (prefix[index].command.target != record.command.target
            || prefix[index]
                .command
                .reviewed_context
                .accepted_target_witness
                != record.command.reviewed_context.accepted_target_witness)
    {
        return Err(EvidenceSubjectAttachmentError::Conflict(
            "active target changed",
        ));
    }
    *state = State::Superseded;
    Ok(())
}

fn revoke(
    record: &ReviewedReferenceRecordV1,
    prior: &AttachmentRecordReferenceV1,
    latest: Option<usize>,
    prefix: &[ReviewedReferenceRecordV1],
    states: &mut [State],
    grant: &mut validate::Grant,
) -> Result<()> {
    let index = predecessor(record, prior, latest, prefix, grant)?;
    let state = states
        .get_mut(index)
        .ok_or(EvidenceSubjectAttachmentError::Invalid("predecessor state"))?;
    if *state != State::Active
        || prefix[index].command.target != record.command.target
        || prefix[index]
            .command
            .reviewed_context
            .accepted_target_witness
            != record.command.reviewed_context.accepted_target_witness
    {
        return Err(EvidenceSubjectAttachmentError::Conflict(
            "revocation historical target",
        ));
    }
    *state = State::Superseded;
    Ok(())
}

fn latest(
    record: &ReviewedReferenceRecordV1,
    prefix: &[ReviewedReferenceRecordV1],
    grant: &mut validate::Grant,
) -> Result<Option<usize>> {
    let mut latest = None;
    for (index, previous) in prefix.iter().enumerate() {
        grant.steps(1)?;
        if previous.command.operation == record.command.operation {
            return Err(EvidenceSubjectAttachmentError::Conflict(
                "duplicate operation",
            ));
        }
        if previous.command.target.public_id == record.command.target.public_id
            && same_subjects(previous, record, grant)?
        {
            latest = Some(index);
        }
    }
    Ok(latest)
}

fn same_subjects(
    left: &ReviewedReferenceRecordV1,
    right: &ReviewedReferenceRecordV1,
    grant: &mut validate::Grant,
) -> Result<bool> {
    let left = left.command.subjects.as_slice();
    let right = right.command.subjects.as_slice();
    if left.len() != right.len() {
        return Ok(false);
    }
    for (left, right) in left.iter().zip(right) {
        grant.steps(1)?;
        if left.subject != right.subject {
            return Ok(false);
        }
    }
    Ok(true)
}

fn predecessor(
    record: &ReviewedReferenceRecordV1,
    prior: &AttachmentRecordReferenceV1,
    latest: Option<usize>,
    prefix: &[ReviewedReferenceRecordV1],
    grant: &mut validate::Grant,
) -> Result<usize> {
    let index = latest.ok_or(EvidenceSubjectAttachmentError::Conflict(
        "missing predecessor chain",
    ))?;
    let previous = prefix
        .get(index)
        .ok_or(EvidenceSubjectAttachmentError::Overflow)?;
    grant.steps(1)?;
    if previous.command.operation != prior.operation || previous.record_digest != prior.digest {
        return Err(EvidenceSubjectAttachmentError::Conflict(
            "predecessor is not exact latest",
        ));
    }
    if previous.command.subjects != record.command.subjects {
        return Err(EvidenceSubjectAttachmentError::Conflict(
            "subject definition changed",
        ));
    }
    retained(previous, record, grant)?;
    Ok(index)
}

fn retained(
    previous: &ReviewedReferenceRecordV1,
    record: &ReviewedReferenceRecordV1,
    grant: &mut validate::Grant,
) -> Result<()> {
    let old = &previous.command.reviewed_context;
    let new = &record.command.reviewed_context;
    retain(old.objects.as_slice(), new.objects.as_slice(), grant)?;
    retain(old.facts.as_slice(), new.facts.as_slice(), grant)?;
    retain(
        old.corroboration.as_slice(),
        new.corroboration.as_slice(),
        grant,
    )?;
    retain(
        old.complete_scope_records.as_slice(),
        new.complete_scope_records.as_slice(),
        grant,
    )?;
    retain(
        old.accepted_target_subjects.as_slice(),
        new.accepted_target_subjects.as_slice(),
        grant,
    )
}

fn retain<T: PartialEq>(old: &[T], new: &[T], grant: &mut validate::Grant) -> Result<()> {
    for expected in old {
        let mut found = false;
        for actual in new {
            grant.steps(1)?;
            if expected == actual {
                found = true;
                break;
            }
        }
        if !found {
            return Err(EvidenceSubjectAttachmentError::Conflict(
                "prior retained context omitted or changed",
            ));
        }
    }
    Ok(())
}

fn overlaps(
    record: &ReviewedReferenceRecordV1,
    prefix: &[ReviewedReferenceRecordV1],
    states: &[State],
    grant: &mut validate::Grant,
) -> Result<()> {
    for (previous, state) in prefix.iter().zip(states) {
        grant.steps(1)?;
        if *state != State::Active {
            continue;
        }
        for left in previous.command.subjects.as_slice() {
            for right in record.command.subjects.as_slice() {
                grant.steps(1)?;
                if left.subject == right.subject {
                    return Err(EvidenceSubjectAttachmentError::Conflict(
                        "overlapping active subject",
                    ));
                }
            }
        }
    }
    Ok(())
}
