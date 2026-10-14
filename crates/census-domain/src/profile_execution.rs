use crate::model::{
    CanonicalJsonError, DiscoveryPriorityV1, ProfileError, ProvisionalInclusionV1, QueryProfileV1,
    RegionalMembershipV1,
};
use crate::UsJurisdiction;

mod components;
mod descriptor;
mod policy;
mod role;

pub use descriptor::{
    profile_contract_descriptor_v1, profile_contract_ref_v1, ContractDescriptorV1,
};
pub use role::ContractRoleV1;

#[derive(Debug, thiserror::Error)]
pub enum ProfileExecutionError {
    #[error("unsupported exact profile contract reference for {role:?}")]
    UnsupportedContractReference { role: ContractRoleV1 },
    #[error("jurisdiction {jurisdiction:?} is outside the current profile execution envelope")]
    UnsupportedJurisdiction { jurisdiction: UsJurisdiction },
    #[error("source-backed regional membership is not supported by the configuration compiler")]
    UnsupportedRegionalMembership,
    #[error("source-backed regional priority is not supported by the configuration compiler")]
    UnsupportedDiscoveryPriority,
    #[error("provisional inclusion is not supported by the configuration compiler")]
    UnsupportedProvisionalInclusion,
    #[error("acquisition through exceeds the configuration compiler's effective as_of cutoff")]
    AcquisitionAfterAsOf,
    #[error("profile contract canonical identity failed: {0}")]
    Canonical(#[from] CanonicalJsonError),
    #[error("profile contract reference construction failed: {0}")]
    ReferenceConstruction(#[from] ProfileError),
}

#[derive(Debug)]
#[must_use]
pub struct CompiledProfileV1<'a> {
    profile: &'a QueryProfileV1,
}

impl<'a> CompiledProfileV1<'a> {
    pub fn profile(&self) -> &'a QueryProfileV1 {
        self.profile
    }
}

pub fn compile_profile(
    profile: &QueryProfileV1,
) -> Result<CompiledProfileV1<'_>, ProfileExecutionError> {
    for role in ContractRoleV1::ALL {
        let expected = profile_contract_ref_v1(role)?;
        if role.reference(profile.input()) != &expected {
            return Err(ProfileExecutionError::UnsupportedContractReference { role });
        }
    }
    check_configuration(profile)?;
    Ok(CompiledProfileV1 { profile })
}

fn check_configuration(profile: &QueryProfileV1) -> Result<(), ProfileExecutionError> {
    let input = profile.input();
    for jurisdiction in &input.jurisdictions {
        if !UsJurisdiction::CENSUS_SCOPE.contains(jurisdiction) {
            return Err(ProfileExecutionError::UnsupportedJurisdiction {
                jurisdiction: *jurisdiction,
            });
        }
    }
    match &input.regional_membership {
        RegionalMembershipV1::AllSelectedJurisdictions => {}
        RegionalMembershipV1::SourceBackedRegion(_) => {
            return Err(ProfileExecutionError::UnsupportedRegionalMembership);
        }
    }
    match &input.discovery_priority {
        DiscoveryPriorityV1::Uniform => {}
        DiscoveryPriorityV1::SourceBackedRegionFirst(_) => {
            return Err(ProfileExecutionError::UnsupportedDiscoveryPriority);
        }
    }
    match input.include_provisional {
        ProvisionalInclusionV1::Exclude => {}
        ProvisionalInclusionV1::Include => {
            return Err(ProfileExecutionError::UnsupportedProvisionalInclusion);
        }
    }
    if input.acquisition_window.through() > &input.as_of {
        return Err(ProfileExecutionError::AcquisitionAfterAsOf);
    }
    Ok(())
}
