use super::*;
use crate::StoreSnapshot;
use census_domain::model::StoreIdentity;

pub(super) fn prepare<'s>(
    view: &StoreSnapshot<'s>,
    identity: &StoreIdentity,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    changed: bool,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PreparedParts<'s>> {
    let mut ledger = Ledger::load(view, identity, grant)?;
    let (approval, retained) = find(&mut ledger, &command, grant)?;
    if let Some(record) = retained {
        return Ok((view.revision(), record, approval, PreparedState::Replay));
    }
    if changed {
        return Err(Error::InventoryMoved.into());
    }
    Err(Error::Corrupt("new attachment requires an inspection").into())
}

pub(super) fn find(
    ledger: &mut Ledger<'_>,
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(Vec<u8>, Option<ReviewedReferenceRecordV1>)> {
    let approval = command
        .approval_bytes_with_grant(4 * 1024 * 1024, grant)
        .map_err(Error::Domain)?;
    let Some(index) = ledger.find_operation(command.operation.as_str(), grant)? else {
        return Ok((approval, None));
    };
    grant.charge_steps(1).map_err(Error::Domain)?;
    let retained = ledger
        .records
        .get(index)
        .ok_or(Error::Corrupt("replay record index"))?;
    if &retained.command != command {
        return Err(Error::OperationConflict.into());
    }
    Ok((approval, Some(ledger.records.remove(index))))
}
