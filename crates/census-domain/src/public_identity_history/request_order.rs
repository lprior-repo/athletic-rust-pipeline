use super::{HistoryError, HistoryRequest};

pub(super) fn normalize(mut request: HistoryRequest) -> Result<HistoryRequest, HistoryError> {
    match &mut request {
        HistoryRequest::Establish { evidence, .. } | HistoryRequest::Bind { evidence, .. } => {
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
