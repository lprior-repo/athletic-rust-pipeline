use super::MAX_REGION_CONTRACT_RECORD_BYTES;
use super::{keys, wire, RegionContractRegistryError as Error, MAX_REGION_CONTRACT_CHUNKS};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::{ContractRefV1, RegionMembershipContractV1, StoreIdentity};
use fjall::{Keyspace, Readable, Snapshot};

pub(super) struct Retained {
    pub(super) payload: RegionMembershipContractV1,
    pub(super) bytes: Vec<u8>,
}

impl Store {
    pub fn region_contract(
        &self,
        reference: &ContractRefV1,
    ) -> StoreResult<RegionMembershipContractV1> {
        read(&self.db.snapshot(), &self.meta, reference)
    }

    pub fn selected_region_contract(
        &self,
        identity: &StoreIdentity,
        reference: &ContractRefV1,
    ) -> StoreResult<RegionMembershipContractV1> {
        self.snapshot()
            .selected_region_contract(identity, reference)
    }
}

pub(crate) fn read(
    snapshot: &Snapshot,
    meta: &Keyspace,
    reference: &ContractRefV1,
) -> StoreResult<RegionMembershipContractV1> {
    match retained(snapshot, meta, reference)? {
        Some(held) => Ok(held.payload),
        None => Err(Error::MissingContract {
            reference: reference.clone(),
        }
        .into()),
    }
}

pub(super) fn retained(
    snapshot: &Snapshot,
    meta: &Keyspace,
    reference: &ContractRefV1,
) -> StoreResult<Option<Retained>> {
    let key = keys::descriptor(reference)?;
    let Some(length) = snapshot
        .size_of(meta, key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
    else {
        return absent(snapshot, meta, reference, &key);
    };
    wire::bounded(
        "descriptor",
        usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?,
        MAX_REGION_CONTRACT_RECORD_BYTES,
    )?;
    let raw = snapshot
        .get(meta, key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| Error::MissingContract {
            reference: reference.clone(),
        })?;
    let descriptor = wire::descriptor(reference, raw.as_ref())?;
    validate_keys(snapshot, meta, reference, &key, descriptor.chunks)?;
    let bytes = reconstruct(snapshot, meta, &key, &descriptor)?;
    let payload = wire::payload(reference, &bytes)?;
    Ok(Some(Retained { payload, bytes }))
}

fn absent(
    snapshot: &Snapshot,
    meta: &Keyspace,
    reference: &ContractRefV1,
    key: &str,
) -> StoreResult<Option<Retained>> {
    if let Some(guard) = snapshot.prefix(meta, keys::chunk_prefix(key)?).next() {
        guard.key().map_err(|source| StoreError::Read { source })?;
        return Err(Error::OrphanPayload {
            reference: reference.clone(),
        }
        .into());
    }
    Ok(None)
}

fn validate_keys(
    snapshot: &Snapshot,
    meta: &Keyspace,
    reference: &ContractRefV1,
    key: &str,
    count: usize,
) -> StoreResult<()> {
    let limit = MAX_REGION_CONTRACT_CHUNKS
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    for guard in snapshot.prefix(meta, keys::chunk_prefix(key)?).take(limit) {
        let raw = guard.key().map_err(|source| StoreError::Read { source })?;
        let index = keys::ordinal(key, raw.as_ref()).ok_or_else(|| Error::InvalidChunkKey {
            reference: reference.clone(),
        })?;
        if index >= count {
            return Err(Error::ExtraPayload {
                reference: reference.clone(),
                index,
                declared: count,
            }
            .into());
        }
    }
    Ok(())
}

fn reconstruct(
    snapshot: &Snapshot,
    meta: &Keyspace,
    key: &str,
    descriptor: &wire::Descriptor,
) -> StoreResult<Vec<u8>> {
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(descriptor.payload_bytes)
        .map_err(|_| Error::AllocationFailed)?;
    for index in 0..descriptor.chunks {
        let remaining = descriptor
            .payload_bytes
            .checked_sub(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        let expected = remaining.min(MAX_REGION_CONTRACT_RECORD_BYTES);
        let raw_key = keys::chunk(key, index)?;
        let length = snapshot
            .size_of(meta, &raw_key)
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| Error::MissingPayload {
                reference: descriptor.reference.clone(),
                index,
            })?;
        let observed = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        chunk_length(descriptor, index, expected, observed)?;
        let raw = snapshot
            .get(meta, &raw_key)
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| Error::MissingPayload {
                reference: descriptor.reference.clone(),
                index,
            })?;
        chunk_length(descriptor, index, expected, raw.len())?;
        bytes.extend_from_slice(raw.as_ref());
    }
    Ok(bytes)
}

fn chunk_length(
    descriptor: &wire::Descriptor,
    index: usize,
    expected: usize,
    observed: usize,
) -> StoreResult<()> {
    if observed != expected {
        return Err(Error::ChunkLength {
            reference: descriptor.reference.clone(),
            index,
            expected,
            observed,
        }
        .into());
    }
    Ok(())
}
