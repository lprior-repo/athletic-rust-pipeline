use super::super::terminal_operation;
use crate::acquisition::facts::{digest, AttemptFacts, ImmutableAttemptResult, ReceiptWitness};
use crate::acquisition::{AcquisitionReadBudget, CaptureError};
use crate::{Receipt, StoreError, StoreResult};
use census_domain::model::{CaptureProtocolVersion, MAX_CAPTURE_RECORD_BYTES};
use fjall::{Keyspace, Readable, Snapshot};

pub(in crate::acquisition::read) fn receipt(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    facts: &AttemptFacts,
    budget: &mut AcquisitionReadBudget,
    protocol: CaptureProtocolVersion,
) -> StoreResult<ReceiptWitness> {
    let operation = terminal_operation(&facts.reservation.attempt, protocol);
    budget.receipt_probe()?;
    let length = snapshot
        .size_of(receipts, operation.as_bytes())
        .map_err(|source| StoreError::Read { source })?;
    if let Some(length) = length {
        let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        if length > MAX_CAPTURE_RECORD_BYTES {
            return Err(CaptureError::RecordTooLarge.into());
        }
        budget.encoded(
            operation
                .len()
                .checked_add(length)
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        budget.present_receipt()?;
    }
    let raw = snapshot
        .get(receipts, operation.as_bytes())
        .map_err(|source| StoreError::Read { source })?;
    match (&facts.terminal, raw) {
        (None, None) => Ok(ReceiptWitness::Absent),
        (Some(terminal), Some(raw)) => decode_receipt_witness(&operation, terminal, &raw, budget),
        _ => Err(CaptureError::CorruptClosure.into()),
    }
}

fn decode_receipt_witness(
    operation: &str,
    terminal: &ImmutableAttemptResult,
    raw: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<ReceiptWitness> {
    if raw.len() > MAX_CAPTURE_RECORD_BYTES {
        return Err(CaptureError::RecordTooLarge.into());
    }
    let receipt: Receipt = {
        crate::acquisition::codec::reserve_decode(raw, budget)?;
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: operation.to_string(),
            source,
        })?
    };
    if receipt.operation != operation
        || receipt.digest != digest(terminal)?.as_str()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(CaptureError::CorruptClosure.into());
    }
    let mut owned = Vec::new();
    budget.reserve_recovery_owned(raw.len())?;
    owned.try_reserve_exact(raw.len()).map_err(|source| {
        CaptureError::allocation_reservation_failed("receipt buffer", Some(raw.len()), source)
    })?;
    owned.extend_from_slice(raw);
    Ok(ReceiptWitness::Present {
        receipt,
        raw: owned,
    })
}
