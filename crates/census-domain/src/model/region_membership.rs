use super::{CanonicalJsonError, ContractRefV1, ProfileDate, ProfileDateRangeV1};
use serde::{Deserialize, Deserializer, Serialize};
use std::num::NonZeroU32;

mod assertion;
mod boundary;
mod context;
mod evidence;
mod resolve;
mod subject;

#[cfg(test)]
mod tests;

pub use assertion::{
    RegionAssertionInputV1, RegionAssertionV1, RegionEffectiveV1, RegionStatementV1,
};
pub use context::{RegionContextV1, RegionKeyV1};
pub use evidence::{RegionEvidenceInputV1, RegionEvidenceV1};
pub use resolve::{
    bind_region_contract_v1, resolve_region_membership_v1, resolve_region_priority_v1,
    BoundRegionContractV1, RegionAssertionDispositionV1, RegionAssertionTraceV1,
    RegionCompositionResolutionV1, RegionMembershipQueryV1, RegionMembershipResolutionV1,
    RegionMembershipTargetV1, RegionMembershipVerdictV1, RegionPriorityResolutionV1,
    RegionPriorityTierV1,
};
pub use subject::{RegionCompositionV1, RegionProgramV1, RegionSchoolV1, RegionSubjectV1};

pub const REGION_MEMBERSHIP_CONTRACT_VERSION: u32 = 1;
pub const MAX_REGION_ASSERTIONS: usize = 4096;
pub const MAX_REGION_JURISDICTIONS: usize = 51;
pub const MAX_REGION_CONSTITUENTS: usize = 16;
pub const MAX_REGION_PAYLOAD_BYTES: usize = 1_048_576;

#[derive(Debug, thiserror::Error)]
pub enum RegionMembershipError {
    #[error("unsupported regional membership payload version {found}")]
    UnsupportedVersion { found: u32 },
    #[error("regional membership revision must be nonzero")]
    ZeroRevision,
    #[error("{field} is malformed or outside its admitted bounds")]
    InvalidField { field: &'static str },
    #[error("{field} contains {observed} raw entries, exceeding {maximum}")]
    TooManyEntries {
        field: &'static str,
        observed: usize,
        maximum: usize,
    },
    #[error("regional payload exceeds its {maximum}-byte ceiling")]
    PayloadTooLarge { maximum: usize },
    #[error("regional payload allocation failed")]
    AllocationFailed,
    #[error("regional reference does not bind this exact retained payload and revision")]
    ReferenceMismatch,
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
    #[error(transparent)]
    Profile(#[from] super::ProfileError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionMembershipContractInputV1 {
    pub version: u32,
    pub revision: u32,
    pub region: RegionKeyV1,
    #[serde(deserialize_with = "boundary::jurisdictions")]
    pub jurisdictions: Vec<crate::UsJurisdiction>,
    pub effective: ProfileDateRangeV1,
    #[serde(deserialize_with = "boundary::assertions")]
    pub assertions: Vec<RegionAssertionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RegionMembershipContractV1(RegionMembershipContractInputV1);

impl RegionMembershipContractV1 {
    pub fn new(mut input: RegionMembershipContractInputV1) -> Result<Self, RegionMembershipError> {
        if input.version != REGION_MEMBERSHIP_CONTRACT_VERSION {
            return Err(RegionMembershipError::UnsupportedVersion {
                found: input.version,
            });
        }
        NonZeroU32::new(input.revision).ok_or(RegionMembershipError::ZeroRevision)?;
        boundary::count(
            "jurisdictions",
            input.jurisdictions.len(),
            MAX_REGION_JURISDICTIONS,
        )?;
        boundary::count("assertions", input.assertions.len(), MAX_REGION_ASSERTIONS)?;
        if input.jurisdictions.is_empty() {
            return Err(RegionMembershipError::InvalidField {
                field: "jurisdictions",
            });
        }
        boundary::retained_bytes(&input)?;
        input.jurisdictions.sort_unstable();
        input.jurisdictions.dedup();
        input.assertions.sort_unstable();
        input.assertions.dedup();
        Ok(Self(input))
    }

    pub fn input(&self) -> &RegionMembershipContractInputV1 {
        &self.0
    }

    pub fn reference(&self) -> Result<ContractRefV1, RegionMembershipError> {
        let digest = super::serialized_digest(self)?;
        Ok(ContractRefV1::new(1, self.0.revision, &digest)?)
    }

    pub fn admits_effective_day(&self, date: &ProfileDate) -> bool {
        self.0.effective.contains(date)
    }

    pub fn admits_jurisdiction(&self, jurisdiction: crate::UsJurisdiction) -> bool {
        self.0.jurisdictions.binary_search(&jurisdiction).is_ok()
    }
}

impl<'de> Deserialize<'de> for RegionMembershipContractV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(RegionMembershipContractInputV1::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

pub fn admit_region_payload_bytes_v1(bytes: &[u8]) -> Result<(), RegionMembershipError> {
    if bytes.len() > MAX_REGION_PAYLOAD_BYTES {
        return Err(RegionMembershipError::PayloadTooLarge {
            maximum: MAX_REGION_PAYLOAD_BYTES,
        });
    }
    Ok(())
}
