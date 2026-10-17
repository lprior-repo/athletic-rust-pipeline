use super::*;
use crate::StoreSnapshot;
use census_domain::evidence_subject_attachment::check_attachment_command_with_grant;
use census_domain::model::StoreIdentity;

pub(super) enum Found {
    New {
        command: CheckedAttachmentCommandV1,
        approval: Vec<u8>,
    },
    Repeated {
        command: CheckedAttachmentCommandV1,
        approval: Vec<u8>,
        approval_digest: AttachmentDigestV1,
        index: usize,
    },
}

pub(super) fn prepare<'s>(
    view: &StoreSnapshot<'s>,
    identity: &StoreIdentity,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    changed: bool,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<PreparedParts<'s>> {
    let ledger = Ledger::load(view, identity, grant)?;
    if let Found::Repeated {
        command,
        approval,
        approval_digest,
        index,
    } = find(&ledger, command, grant)?
    {
        let input = ReplayPrepared {
            command,
            approval_digest,
            ledger,
            index,
        };
        return Ok((
            view.revision(),
            approval,
            PreparedState::Replay(Box::new(input)),
        ));
    }
    if changed {
        return Err(Error::InventoryMoved.into());
    }
    Err(Error::Corrupt("new attachment requires an inspection").into())
}

pub(super) fn find(
    ledger: &Ledger<'_>,
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
    grant.charge_bytes(approval.len()).map_err(Error::Domain)?;
    grant.charge_bytes(64).map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let approval_digest =
        AttachmentDigestV1::parse(retained.approval_digest.as_str()).map_err(Error::Domain)?;
    Ok(Found::Repeated {
        command,
        approval,
        approval_digest,
        index,
    })
}
