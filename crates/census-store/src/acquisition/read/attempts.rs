use super::closure;
use crate::acquisition::{facts::AttemptFacts, AcquisitionReadBudget, CaptureError};
use crate::{StoreError, StoreResult};
use census_domain::model::{
    AcquisitionAttemptId, AcquisitionWorkId, AcquisitionWorkV1, AttemptOrdinal,
    CaptureProtocolVersion,
};
use fjall::{Keyspace, Snapshot};
use std::collections::BTreeMap;

const ATTEMPT_FACTS_HINT_BYTES: usize = 3 * std::mem::size_of::<AttemptFacts>();

pub(super) fn reconstruct(
    rows: &mut BTreeMap<String, Vec<u8>>,
    work: &AcquisitionWorkV1,
    id: &AcquisitionWorkId,
    snapshot: &Snapshot,
    receipts: &Keyspace,
    budget: &mut AcquisitionReadBudget,
    protocol: CaptureProtocolVersion,
) -> StoreResult<Vec<AttemptFacts>> {
    let bytes = std::mem::size_of::<[AttemptFacts; 3]>()
        .checked_add(384)
        .ok_or(StoreError::CounterOverflow)?;
    budget.reserve_recovery_owned(bytes)?;
    let mut attempts = Vec::new();
    attempts.try_reserve_exact(3).map_err(|source| {
        CaptureError::allocation_reservation_failed(
            "attempt facts buffer",
            Some(ATTEMPT_FACTS_HINT_BYTES),
            source,
        )
    })?;
    for ordinal in 1..=3 {
        let ordinal = AttemptOrdinal::new(ordinal).map_err(CaptureError::Domain)?;
        let attempt = read_attempt_id(work, id, ordinal, protocol)?;
        let Some(mut facts) = closure::attempt(rows, &attempt, protocol, budget)? else {
            break;
        };
        facts.receipt = closure::receipt(snapshot, receipts, &facts, budget, protocol)?;
        attempts.push(facts);
    }
    Ok(attempts)
}

fn read_attempt_id(
    work: &AcquisitionWorkV1,
    id: &AcquisitionWorkId,
    ordinal: AttemptOrdinal,
    protocol: CaptureProtocolVersion,
) -> StoreResult<AcquisitionAttemptId> {
    let (identity, work_id) = match protocol {
        CaptureProtocolVersion::V1 => (work.slot().store_identity().clone(), id.clone()),
        CaptureProtocolVersion::V2 => (
            work.slot().store_identity().try_clone().map_err(|source| {
                CaptureError::allocation_reservation_failed("attempt store identity", None, source)
            })?,
            id.try_clone().map_err(|source| {
                CaptureError::allocation_reservation_failed("attempt work id", None, source)
            })?,
        ),
    };
    Ok(AcquisitionAttemptId::new(identity, work_id, ordinal))
}
