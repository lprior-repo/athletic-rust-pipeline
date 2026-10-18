use super::{
    check_selection_configuration, selected_qualification_contract_ref_v4, ContractRoleV1,
    ProfileExecutionError,
};
use crate::model::{serialized_digest, ContractRefV1, QueryProfileV1};
use serde::Serialize;

#[derive(Serialize)]
struct Descriptor {
    version: u32,
    revision: u32,
    role: ContractRoleV1,
    scope: &'static str,
    component: &'static str,
}

#[derive(Debug)]
#[must_use]
pub struct SupportedQualificationProfileV5<'a> {
    profile: &'a QueryProfileV1,
}

impl<'a> SupportedQualificationProfileV5<'a> {
    pub fn profile(&self) -> &'a QueryProfileV1 {
        self.profile
    }
}

pub fn selected_qualification_contract_ref_v5(
    role: ContractRoleV1,
) -> Result<ContractRefV1, ProfileExecutionError> {
    let component = match role {
        ContractRoleV1::Affiliation => "reviewed-roster-exact-utc-day-complete-connected-inventory-accepted-secondary-person-bridge-v1",
        ContractRoleV1::Publication => "selected-artifact8-frozen-graph-envelope-v3-reviewed-roster-descriptor-v3",
        _ => return selected_qualification_contract_ref_v4(role),
    };
    let digest = serialized_digest(&Descriptor {
        version: 1,
        revision: 6,
        role,
        scope: "retained-identity-graph-v3-source-lifecycle8-accepted-secondary-person-bridge-v1",
        component,
    })?;
    Ok(ContractRefV1::new(1, 6, &digest)?)
}

pub fn validate_selected_qualification_profile_v5(
    profile: &QueryProfileV1,
) -> Result<SupportedQualificationProfileV5<'_>, ProfileExecutionError> {
    for role in ContractRoleV1::ALL {
        if role.reference(profile.input()) != &selected_qualification_contract_ref_v5(role)? {
            return Err(ProfileExecutionError::UnsupportedContractReference { role });
        }
    }
    check_selection_configuration(profile)?;
    Ok(SupportedQualificationProfileV5 { profile })
}
