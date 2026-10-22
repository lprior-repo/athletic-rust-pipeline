use super::super::super::{codec, source_error, RosterSourceIssuanceStoreError as Error};
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use fjall::{Keyspace, Readable, Snapshot};

pub(in super::super) fn read(
    snapshot: &Snapshot,
    keyspace: &Keyspace,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(2).map_err(source_error)?;
    let length = snapshot
        .size_of(keyspace, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("withdrawal dependency absent"))?;
    let length = usize::try_from(length).map_err(|_| Error::BudgetExceeded)?;
    let cap = if key.starts_with(super::super::OBJECT_PREFIX) {
        census_domain::roster_affiliation_authority::MAX_ROSTER_RETAINED_OBJECT_BYTES
    } else if key.starts_with(super::super::RECORD_PREFIX) {
        codec::MAX_REVIEW_COMPOSITE_BYTES
    } else {
        codec::MAX_BYTES
    };
    if length == 0 || length > cap {
        return Err(Error::BudgetExceeded.into());
    }
    let charged = length.checked_add(key.len()).ok_or(Error::BudgetExceeded)?;
    grant.charge_bytes(charged).map_err(source_error)?;
    let raw = snapshot
        .get(keyspace, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("withdrawal dependency disappeared"))?;
    if raw.len() != length {
        return Err(Error::Corrupt("withdrawal dependency length changed").into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| Error::BudgetExceeded)?;
    bytes.extend_from_slice(&raw);
    Ok(bytes)
}
