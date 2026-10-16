use super::*;
use crate::StoreSnapshot;
use census_domain::evidence_subject_attachment::{
    check_attachment_command_with_grant, CheckedAttachmentCommandV1,
};
use census_domain::model::StoreIdentity;

pub(super) enum Found {
    New {
        command: CheckedAttachmentCommandV1,
        approval: Vec<u8>,
    },
    Repeated {
        record: CheckedReviewedReferenceRecordV1,
        approval: Vec<u8>,
    },
}

pub(super) fn prepare<'s>(
    view: &StoreSnapshot<'s>,
    identity: &StoreIdentity,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    changed: bool,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PreparedParts<'s>> {
    let mut ledger = Ledger::load(view, identity, grant)?;
    if let Found::Repeated { record, approval } = find(&mut ledger, command, grant)? {
        return Ok((view.revision(), record, approval, PreparedState::Replay));
    }
    if changed {
        return Err(Error::InventoryMoved.into());
    }
    Err(Error::Corrupt("new attachment requires an inspection").into())
}

pub(super) fn find(
    ledger: &mut Ledger<'_>,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Found> {
    let command = check_attachment_command_with_grant(command, grant).map_err(Error::Domain)?;
    let approval = command
        .approval_bytes_with_grant(4 * 1024 * 1024, grant)
        .map_err(Error::Domain)?;
    let Some(index) = ledger.find_operation(command.operation.as_str(), grant)? else {
        return Ok(Found::New { command, approval });
    };
    grant.charge_steps(1).map_err(Error::Domain)?;
    let retained = ledger
        .records
        .get(index)
        .ok_or(Error::Corrupt("replay record index"))?;
    if &retained.command != command.command() {
        return Err(Error::OperationConflict.into());
    }
    let record = ledger
        .records
        .remove_with_grant(index, grant)
        .map_err(Error::Domain)?;
    Ok(Found::Repeated { record, approval })
}
