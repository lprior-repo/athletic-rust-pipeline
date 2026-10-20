use crate::{AcquisitionReadBudget, CaptureError, StoreError, StoreFormat, StoreResult};
use fjall::{Keyspace, Readable, Snapshot};

pub(crate) fn read_capture_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<StoreFormat> {
    Ok(StoreFormat {
        schema_version: number(snapshot, meta, super::STORE_VERSION, budget)?,
        key_format: number(snapshot, meta, super::KEY_FORMAT, budget)?,
        lifecycle_revision: number(snapshot, meta, super::LIFECYCLE_REVISION_KEY, budget)?,
        created_by: text(snapshot, meta, super::CREATED_BY, budget)?,
        created_at: text(snapshot, meta, super::CREATED_AT, budget)?,
        migration_target: number(snapshot, meta, super::MIGRATING_TO, budget)?,
    })
}

fn number(
    snapshot: &Snapshot,
    meta: &Keyspace,
    key: &str,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<u32>> {
    text(snapshot, meta, key, budget)?
        .map(|value| crate::meta::parse_u32(key, &value))
        .transpose()
}

fn text(
    snapshot: &Snapshot,
    meta: &Keyspace,
    key: &str,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<String>> {
    budget.admission_probe()?;
    let Some(length) = snapshot
        .size_of(meta, key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if length > 65_536 {
        return Err(CaptureError::ReadBudgetLimitExceeded {
            resource: "admission encoded bytes",
            required: u64::try_from(length).map_err(|_| StoreError::CounterOverflow)?,
            limit: 65_536,
        }
        .into());
    }
    budget.admission_value(key.len(), length)?;
    let raw = snapshot
        .get(meta, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(CaptureError::CaptureMarkerMismatch)?;
    let value = std::str::from_utf8(&raw).map_err(|_| StoreError::SchemaUnknown {
        detail: format!("{key} is not utf-8"),
    })?;
    budget.reserve_recovery_owned(value.len())?;
    let mut owned = String::new();
    owned.try_reserve_exact(value.len()).map_err(|source| {
        CaptureError::allocation_reservation_failed(
            "capture metadata string",
            Some(value.len()),
            source,
        )
    })?;
    owned.push_str(value);
    Ok(Some(owned))
}
