use super::{AcquisitionReadBudget, CaptureError};
use crate::clock::{Clock, SystemClock};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::{MAX_CAPTURE_RECORD_BYTES, SOURCE_ACQUISITION_PHASE};
use fjall::{Keyspace, OwnedWriteBatch, Readable, Snapshot};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<T> {
    key: String,
    at: String,
    payload: T,
}

pub(super) fn decode<T: DeserializeOwned>(key: &str, raw: &[u8]) -> StoreResult<T> {
    if raw.len() > MAX_CAPTURE_RECORD_BYTES {
        return Err(CaptureError::RecordTooLarge.into());
    }
    let value: Envelope<T> = serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
        key: key.to_string(),
        source,
    })?;
    if value.key != key || value.at.is_empty() || value.at.len() > 64 {
        return Err(CaptureError::CorruptClosure.into());
    }
    Ok(value.payload)
}

pub(super) fn raw(
    snapshot: &Snapshot,
    journal: &Keyspace,
    key: &str,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<Vec<u8>>> {
    let physical = Store::journal_key(SOURCE_ACQUISITION_PHASE, key);
    let Some(length) = snapshot
        .size_of(journal, &physical)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if length > MAX_CAPTURE_RECORD_BYTES {
        return Err(CaptureError::RecordTooLarge.into());
    }
    budget.row(physical.len(), length)?;
    let bytes = snapshot
        .get(journal, &physical)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(CaptureError::CorruptClosure)?;
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(length)
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "allocation",
        })?;
    owned.extend_from_slice(&bytes);
    Ok(Some(owned))
}

pub(super) fn get<T: DeserializeOwned>(
    snapshot: &Snapshot,
    journal: &Keyspace,
    key: &str,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<T>> {
    raw(snapshot, journal, key, budget)?
        .map(|bytes| decode(key, &bytes))
        .transpose()
}

pub(super) fn stage<T: Serialize>(
    store: &Store,
    batch: &mut OwnedWriteBatch,
    key: &str,
    payload: &T,
) -> StoreResult<()> {
    let physical = Store::journal_key(SOURCE_ACQUISITION_PHASE, key);
    if store
        .db
        .snapshot()
        .contains_key(&store.journal, &physical)
        .map_err(|source| StoreError::Read { source })?
    {
        return Err(CaptureError::AttemptTerminalConflict.into());
    }
    let encoded = crate::records::codec::encode_canonical(&Envelope {
        key: key.to_string(),
        at: SystemClock.today_iso8601(),
        payload,
    })?;
    batch.insert(&store.journal, physical, encoded);
    Ok(())
}

pub(super) fn repeat(store: &Store) -> StoreResult<()> {
    store
        .db
        .persist(fjall::PersistMode::SyncData)
        .map_err(|source| StoreError::Flush { source })
}

pub(super) fn commit(batch: OwnedWriteBatch) -> StoreResult<()> {
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

pub(crate) fn reject_generic_phase(phase: &str) -> StoreResult<()> {
    if phase.starts_with("source_acquisition_attempt_") {
        return Err(CaptureError::CaptureQualificationOptInRequired.into());
    }
    Ok(())
}
