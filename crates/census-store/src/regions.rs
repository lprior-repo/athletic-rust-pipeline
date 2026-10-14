use census_domain::model::{ContractRefV1, RegionMembershipContractV1, RegionMembershipError};
use serde::{Deserialize, Serialize};

pub(crate) mod read;
mod registration;

pub const MAX_REGION_RECORD_BYTES: usize = census_domain::model::MAX_REGION_PAYLOAD_BYTES + 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionRegistration {
    Registered(ContractRefV1),
    Repeated(ContractRefV1),
}

#[derive(Debug, thiserror::Error)]
pub enum RegionRegistryError {
    #[error("regional artifact {reference:?} is not retained")]
    Missing { reference: ContractRefV1 },
    #[error("regional artifact {what} version {found} is unsupported")]
    UnsupportedVersion { what: &'static str, found: u32 },
    #[error("regional artifact occupies {bytes} bytes, past its {max}-byte ceiling")]
    Oversized { bytes: usize, max: usize },
    #[error("regional artifact does not bind its requested immutable reference")]
    ReferenceMismatch,
    #[error("regional artifact reference already names a different payload")]
    ImmutableConflict,
    #[error("regional artifact serialization failed: {0}")]
    Canonical(#[from] census_domain::model::CanonicalJsonError),
    #[error("regional artifact record is malformed: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] RegionMembershipError),
    #[error(transparent)]
    Store(#[from] crate::StoreError),
}

#[derive(Serialize)]
struct Envelope<'a> {
    version: u32,
    reference: &'a ContractRefV1,
    payload: &'a RegionMembershipContractV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeInput {
    version: u32,
    reference: ContractRefV1,
    payload: RegionMembershipContractV1,
}

#[cfg(test)]
mod tests;
