use super::{HistoryError, HistoryRequest};

pub(super) fn normalize(mut request: HistoryRequest) -> Result<HistoryRequest, HistoryError> {
    match &mut request {
        HistoryRequest::Establish { evidence, .. }
        | HistoryRequest::Bind { evidence, .. }
        | HistoryRequest::BindCandidateMembership { evidence, .. } => {
            ordered(&mut evidence.members, "evidence members")?;
        }
        HistoryRequest::EstablishMatchedGroup { bindings, evidence } => {
            bindings.sort_by(|left, right| {
                left.candidate
                    .cmp(&right.candidate)
                    .then(left.id.cmp(&right.id))
            });
            ordered(&mut evidence.members, "evidence members")?;
        }
        HistoryRequest::Merge {
            members, evidence, ..
        } => {
            ordered(members, "merge members")?;
            ordered(&mut evidence.members, "evidence members")?;
        }
        HistoryRequest::Split(split) => {
            ordered(&mut split.revoke_merges, "revocations")?;
            ordered(&mut split.unresolved_bindings, "unresolved bindings")?;
            ordered(&mut split.affected_record_keys, "affected records")?;
            ordered(&mut split.evidence.members, "evidence members")?;
            split.partitions.iter_mut().try_for_each(|partition| {
                ordered(&mut partition.binding_ids, "partition bindings")
            })?;
            split
                .partitions
                .sort_by(|left, right| left.binding_ids.cmp(&right.binding_ids));
        }
        HistoryRequest::Revoke(revocation) => {
            ordered(&mut revocation.revoke_merges, "revocations")?;
            ordered(&mut revocation.affected_record_keys, "affected records")?;
            ordered(&mut revocation.evidence.members, "evidence members")?;
        }
    }
    Ok(request)
}

fn ordered<T: Ord>(values: &mut [T], field: &'static str) -> Result<(), HistoryError> {
    values.sort();
    if values
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left == right))
    {
        return Err(HistoryError::Duplicate(field));
    }
    Ok(())
}

pub(super) fn require_canonical(request: &HistoryRequest) -> Result<(), HistoryError> {
    match request {
        HistoryRequest::Establish { evidence, .. }
        | HistoryRequest::Bind { evidence, .. }
        | HistoryRequest::BindCandidateMembership { evidence, .. } => {
            monotonic(&evidence.members, "evidence members")?;
        }
        HistoryRequest::EstablishMatchedGroup { bindings, evidence } => {
            for pair in bindings.windows(2) {
                let order = pair[0]
                    .candidate
                    .cmp(&pair[1].candidate)
                    .then_with(|| pair[0].id.cmp(&pair[1].id));
                if order.is_gt() {
                    return Err(HistoryError::Invalid("noncanonical request order"));
                }
            }
            monotonic(&evidence.members, "evidence members")?;
        }
        HistoryRequest::Merge {
            members, evidence, ..
        } => {
            monotonic(members, "merge members")?;
            monotonic(&evidence.members, "evidence members")?;
        }
        HistoryRequest::Split(split) => {
            monotonic(&split.revoke_merges, "revocations")?;
            monotonic(&split.unresolved_bindings, "unresolved bindings")?;
            monotonic(&split.affected_record_keys, "affected records")?;
            monotonic(&split.evidence.members, "evidence members")?;
            for partition in &split.partitions {
                monotonic(&partition.binding_ids, "partition bindings")?;
            }
            if split
                .partitions
                .windows(2)
                .any(|pair| pair[0].binding_ids > pair[1].binding_ids)
            {
                return Err(HistoryError::Invalid("noncanonical request order"));
            }
        }
        HistoryRequest::Revoke(revocation) => {
            monotonic(&revocation.revoke_merges, "revocations")?;
            monotonic(&revocation.affected_record_keys, "affected records")?;
            monotonic(&revocation.evidence.members, "evidence members")?;
        }
    }
    Ok(())
}

fn monotonic<T: Ord>(values: &[T], field: &'static str) -> Result<(), HistoryError> {
    for pair in values.windows(2) {
        match pair[0].cmp(&pair[1]) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => return Err(HistoryError::Duplicate(field)),
            std::cmp::Ordering::Greater => {
                return Err(HistoryError::Invalid("noncanonical request order"));
            }
        }
    }
    Ok(())
}
