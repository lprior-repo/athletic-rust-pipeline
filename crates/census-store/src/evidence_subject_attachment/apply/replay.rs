use super::*;
use crate::evidence_subject_attachment::prepare::ReplayPrepared;
use crate::StoreSnapshot;
use census_domain::evidence_subject_attachment::{
    CheckedReviewedReferenceRecordV1, ReviewedReferenceRecordV1,
};
use census_domain::model::StoreIdentity;

pub(super) fn apply<'s>(
    guard: AttachmentCommitGuard<'s>,
    view: &StoreSnapshot<'s>,
    identity: &StoreIdentity,
    input: ReplayPrepared<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    let ReplayPrepared {
        command,
        approval_digest,
        mut ledger,
        index,
    } = input;
    if !ledger.refresh(guard.store(), view, identity, grant)? {
        let record = ledger
            .records
            .remove_with_grant(index, grant)
            .map_err(Error::Domain)?;
        drop(ledger);
        drop(command);
        return fresh(view, identity, record, false, grant);
    }
    let Some(found) = ledger.find_operation(command.operation.as_str(), grant)? else {
        return Err(Error::Corrupt("replay-only prepared record disappeared").into());
    };
    grant.charge_steps(1).map_err(Error::Domain)?;
    let record = ledger
        .records
        .get(found)
        .ok_or(Error::Corrupt("replay record missing"))?;
    if found != index
        || &record.command != command.command()
        || record.approval_digest != approval_digest
    {
        return Err(Error::OperationConflict.into());
    }
    receipt(&ledger, found)
}

pub(super) fn fresh(
    view: &StoreSnapshot<'_>,
    identity: &StoreIdentity,
    expected: CheckedReviewedReferenceRecordV1,
    changed: bool,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Application> {
    let ledger = Ledger::load(view, identity, grant)?;
    if let Some(application) = repeated(&ledger, &expected, !changed, grant)? {
        return Ok(application);
    }
    if changed {
        return Err(Error::InventoryMoved.into());
    }
    Err(Error::Corrupt("replay-only prepared record disappeared").into())
}

pub(super) fn repeated(
    ledger: &Ledger<'_>,
    expected: &ReviewedReferenceRecordV1,
    exact_record: bool,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<Application>> {
    let Some(index) = ledger.find_operation(expected.command.operation.as_str(), grant)? else {
        return Ok(None);
    };
    grant.charge_steps(1).map_err(Error::Domain)?;
    let record = ledger
        .records
        .get(index)
        .ok_or(Error::Corrupt("replay record missing"))?;
    if record.command != expected.command
        || record.approval_digest != expected.approval_digest
        || (exact_record && record != expected)
    {
        return Err(Error::OperationConflict.into());
    }
    receipt(ledger, index).map(Some)
}

fn receipt(ledger: &Ledger<'_>, index: usize) -> StoreResult<Application> {
    Ok(Application::Repeated(
        ledger
            .receipts
            .get(index)
            .ok_or(Error::Corrupt("replay receipt missing"))?
            .clone(),
    ))
}
