use super::*;

pub(super) fn retained(
    ledger: &super::ledger::Ledger,
    command: &ResultCalendarReviewCommandV1,
    approval_digest: &ResultCalendarDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<crate::Receipt> {
    grant
        .charge_steps(ledger.records.len())
        .map_err(grant_error)?;
    let index = ledger
        .records
        .iter()
        .position(|record| record.command.operation == command.operation)
        .ok_or(Error::OperationConflict)?;
    let record = ledger
        .records
        .get(index)
        .ok_or(Error::Corrupt("replay record missing"))?;
    let receipt = ledger
        .receipts
        .get(index)
        .ok_or(Error::Corrupt("replay receipt missing"))?;
    if record.command != *command || record.approval_digest != *approval_digest {
        return Err(Error::OperationConflict.into());
    }
    let raw = super::super::codec::encode(receipt, grant)?;
    super::super::codec::decode(&raw, grant)
}
