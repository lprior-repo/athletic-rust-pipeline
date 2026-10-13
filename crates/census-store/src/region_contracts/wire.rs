use super::{RegionContractRegistryError as Error, RegionDescriptorProblem as Problem};
use super::{MAX_REGION_CONTRACT_CHUNKS, MAX_REGION_CONTRACT_RECORD_BYTES};
use crate::StoreResult;
use census_domain::model::{
    admit_region_payload_bytes_v1, ContractRefV1, RegionMembershipContractInputV1,
    RegionMembershipContractV1, MAX_REGION_PAYLOAD_BYTES,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Descriptor {
    pub(super) version: u32,
    pub(super) reference: ContractRefV1,
    pub(super) encoding: String,
    pub(super) payload_bytes: usize,
    pub(super) chunk_bytes: usize,
    pub(super) chunks: usize,
}

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Deserialize)]
struct ReferenceHeader {
    reference: VersionHeader,
}

pub(super) fn descriptor(reference: &ContractRefV1, bytes: &[u8]) -> StoreResult<Descriptor> {
    bounded("descriptor", bytes.len(), MAX_REGION_CONTRACT_RECORD_BYTES)?;
    let header: VersionHeader = parse(reference, "descriptor", bytes)?;
    version("region descriptor", header.version)?;
    let header: ReferenceHeader = parse(reference, "descriptor", bytes)?;
    version("region descriptor reference", header.reference.version)?;
    let held: Descriptor = parse(reference, "descriptor", bytes)?;
    if &held.reference != reference {
        return Err(Error::DescriptorReferenceMismatch {
            requested: reference.clone(),
            claimed: held.reference,
        }
        .into());
    }
    validate_descriptor(&held)?;
    Ok(held)
}

fn validate_descriptor(held: &Descriptor) -> StoreResult<()> {
    bounded("payload", held.payload_bytes, MAX_REGION_PAYLOAD_BYTES)?;
    let problem = if held.payload_bytes == 0 {
        Some(Problem::EmptyPayload)
    } else if held.encoding != "canonical_json" {
        Some(Problem::Encoding)
    } else if held.chunk_bytes != MAX_REGION_CONTRACT_RECORD_BYTES {
        Some(Problem::ChunkSize)
    } else if held.chunks > MAX_REGION_CONTRACT_CHUNKS
        || held.chunks
            != held
                .payload_bytes
                .div_ceil(MAX_REGION_CONTRACT_RECORD_BYTES)
    {
        Some(Problem::ChunkCount)
    } else {
        None
    };
    match problem {
        Some(problem) => Err(Error::InvalidDescriptor {
            reference: held.reference.clone(),
            problem,
        }
        .into()),
        None => Ok(()),
    }
}

pub(super) fn payload(
    reference: &ContractRefV1,
    bytes: &[u8],
) -> StoreResult<RegionMembershipContractV1> {
    admit_region_payload_bytes_v1(bytes).map_err(Error::InvalidPayload)?;
    let header: VersionHeader = parse(reference, "payload", bytes)?;
    version("region payload", header.version)?;
    let input: RegionMembershipContractInputV1 = parse(reference, "payload", bytes)?;
    let payload = RegionMembershipContractV1::new(input).map_err(Error::InvalidPayload)?;
    let computed = payload.reference().map_err(Error::InvalidPayload)?;
    if &computed != reference {
        return Err(Error::ImmutableConflict {
            requested: reference.clone(),
            computed,
        }
        .into());
    }
    let digest = crate::journal::object_digest(bytes);
    if digest != reference.digest() {
        return Err(Error::PayloadDigestMismatch {
            reference: reference.clone(),
            computed: digest,
        }
        .into());
    }
    if encode_payload(&payload)? != bytes {
        return Err(Error::NoncanonicalPayload {
            reference: reference.clone(),
        }
        .into());
    }
    Ok(payload)
}

pub(super) fn encode_payload(payload: &RegionMembershipContractV1) -> StoreResult<Vec<u8>> {
    let bytes = crate::records::codec::encode_canonical_bounded(payload, MAX_REGION_PAYLOAD_BYTES)?;
    admit_region_payload_bytes_v1(&bytes).map_err(Error::InvalidPayload)?;
    Ok(bytes)
}

pub(super) fn bounded(what: &'static str, bytes: usize, max: usize) -> StoreResult<()> {
    if bytes > max {
        return Err(Error::Oversized { what, bytes, max }.into());
    }
    Ok(())
}

fn version(what: &'static str, found: u32) -> StoreResult<()> {
    if found != 1 {
        return Err(Error::UnsupportedVersion { what, found }.into());
    }
    Ok(())
}

fn parse<'de, T: Deserialize<'de>>(
    reference: &ContractRefV1,
    what: &'static str,
    bytes: &'de [u8],
) -> StoreResult<T> {
    serde_json::from_slice(bytes).map_err(|source| {
        Error::Malformed {
            reference: reference.clone(),
            what,
            source,
        }
        .into()
    })
}
