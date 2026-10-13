use census_domain::model::{ContractRefV1, RegionMembershipError, StoreIdentity};

mod keys;
pub(crate) mod read;
mod registration;
mod wire;

#[cfg(test)]
mod tests;

pub const MAX_REGION_CONTRACT_RECORD_BYTES: usize = crate::records::codec::MAX_RECORD_BYTES;
pub const MAX_REGION_CONTRACT_CHUNKS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionContractRegistration {
    Registered(ContractRefV1),
    Repeated(ContractRefV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionDescriptorProblem {
    EmptyPayload,
    ChunkSize,
    ChunkCount,
    Encoding,
}

#[derive(Debug, thiserror::Error)]
pub enum RegionContractRegistryError {
    #[error("region contract {reference:?} is missing")]
    MissingContract { reference: ContractRefV1 },
    #[error("region contract {reference:?} is missing payload chunk {index}")]
    MissingPayload {
        reference: ContractRefV1,
        index: usize,
    },
    #[error("region contract {reference:?} has chunks without a descriptor")]
    OrphanPayload { reference: ContractRefV1 },
    #[error("region contract {reference:?} has malformed {what}: {source}")]
    Malformed {
        reference: ContractRefV1,
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported {what} version {found}")]
    UnsupportedVersion { what: &'static str, found: u32 },
    #[error("region {what} holds {bytes} bytes, exceeding {max}")]
    Oversized {
        what: &'static str,
        bytes: usize,
        max: usize,
    },
    #[error("region contract {reference:?} has invalid descriptor: {problem:?}")]
    InvalidDescriptor {
        reference: ContractRefV1,
        problem: RegionDescriptorProblem,
    },
    #[error("region descriptor for {requested:?} claims {claimed:?}")]
    DescriptorReferenceMismatch {
        requested: ContractRefV1,
        claimed: ContractRefV1,
    },
    #[error("region contract {reference:?} has an invalid chunk key")]
    InvalidChunkKey { reference: ContractRefV1 },
    #[error("region contract {reference:?} declares {declared} chunks but contains chunk {index}")]
    ExtraPayload {
        reference: ContractRefV1,
        index: usize,
        declared: usize,
    },
    #[error(
        "region contract {reference:?} chunk {index} holds {observed} bytes, expected {expected}"
    )]
    ChunkLength {
        reference: ContractRefV1,
        index: usize,
        expected: usize,
        observed: usize,
    },
    #[error("region contract {reference:?} retained bytes digest to {computed}")]
    PayloadDigestMismatch {
        reference: ContractRefV1,
        computed: String,
    },
    #[error("region contract {requested:?} conflicts with payload reference {computed:?}")]
    ImmutableConflict {
        requested: ContractRefV1,
        computed: ContractRefV1,
    },
    #[error("region contract {reference:?} contains noncanonical payload bytes")]
    NoncanonicalPayload { reference: ContractRefV1 },
    #[error("region contract selected store {expected:?} differs from {actual:?}")]
    ForeignStore {
        expected: StoreIdentity,
        actual: StoreIdentity,
    },
    #[error("region registry allocation failed")]
    AllocationFailed,
    #[error(transparent)]
    InvalidPayload(#[from] RegionMembershipError),
}

pub(crate) fn ensure_store(
    expected: &StoreIdentity,
    actual: &StoreIdentity,
) -> crate::StoreResult<()> {
    if expected != actual {
        return Err(RegionContractRegistryError::ForeignStore {
            expected: expected.clone(),
            actual: actual.clone(),
        }
        .into());
    }
    Ok(())
}
