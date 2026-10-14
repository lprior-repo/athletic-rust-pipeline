use super::{EnvelopeInput, RegionRegistryError, MAX_REGION_RECORD_BYTES};
use crate::{Store, StoreError};
use census_domain::model::{ContractRefV1, RegionMembershipContractV1};
use serde::Deserialize;

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Deserialize)]
struct ReferenceHeader {
    reference: ContractRefV1,
}

#[derive(Deserialize)]
struct PayloadHeader {
    payload: PayloadVersion,
}

#[derive(Deserialize)]
struct PayloadVersion {
    version: u32,
}

pub(super) fn key(reference: &ContractRefV1) -> String {
    format!(
        "region-contracts:v1:{}:{}",
        reference.revision(),
        reference.digest()
    )
}

impl Store {
    pub fn region_contract(
        &self,
        reference: &ContractRefV1,
    ) -> Result<RegionMembershipContractV1, RegionRegistryError> {
        self.snapshot().region_contract(reference)
    }
}

pub(crate) fn retained(
    snapshot: &fjall::Snapshot,
    meta: &fjall::Keyspace,
    reference: &ContractRefV1,
) -> Result<Option<RegionMembershipContractV1>, RegionRegistryError> {
    use fjall::Readable;
    let record_key = key(reference);
    let Some(length) = snapshot
        .size_of(meta, record_key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > MAX_REGION_RECORD_BYTES {
        return Err(RegionRegistryError::Oversized {
            bytes,
            max: MAX_REGION_RECORD_BYTES,
        });
    }
    let raw = snapshot
        .get(meta, record_key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| RegionRegistryError::Missing {
            reference: reference.clone(),
        })?;
    decode(reference, raw.as_ref()).map(Some)
}

pub(crate) fn decode(
    expected: &ContractRefV1,
    bytes: &[u8],
) -> Result<RegionMembershipContractV1, RegionRegistryError> {
    if bytes.len() > MAX_REGION_RECORD_BYTES {
        return Err(RegionRegistryError::Oversized {
            bytes: bytes.len(),
            max: MAX_REGION_RECORD_BYTES,
        });
    }
    let header: VersionHeader = parse(bytes)?;
    version("region artifact envelope", header.version)?;
    let ref_header: ReferenceHeader = parse(bytes)?;
    if &ref_header.reference != expected {
        return Err(RegionRegistryError::ReferenceMismatch);
    }
    let payload_header: PayloadHeader = parse(bytes)?;
    version("region membership contract", payload_header.payload.version)?;
    let envelope: EnvelopeInput = parse(bytes)?;
    version("region artifact envelope", envelope.version)?;
    if &envelope.reference != expected {
        return Err(RegionRegistryError::ReferenceMismatch);
    }
    let contract = envelope.payload;
    let computed = contract.reference()?;
    if &computed != expected {
        return Err(RegionRegistryError::ReferenceMismatch);
    }
    Ok(contract)
}

fn version(what: &'static str, found: u32) -> Result<(), RegionRegistryError> {
    if found != 1 {
        return Err(RegionRegistryError::UnsupportedVersion { what, found });
    }
    Ok(())
}

fn parse<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, RegionRegistryError> {
    serde_json::from_slice(bytes).map_err(RegionRegistryError::Malformed)
}
