use super::{
    check_selection_configuration, selected_qualification_contract_ref_v2,
    selected_qualification_contract_ref_v3, selected_qualification_contract_ref_v5,
    validate_selected_qualification_profile_v5, ContractRoleV1, ProfileExecutionError,
};
use crate::model::{serialized_digest, ContractRefV1, EventSemantics, QueryProfileV1};
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
pub struct SupportedQualificationProfileV4<'a> {
    profile: &'a QueryProfileV1,
}

impl<'a> SupportedQualificationProfileV4<'a> {
    pub fn profile(&self) -> &'a QueryProfileV1 {
        self.profile
    }
}

pub fn selected_qualification_contract_ref_v4(
    role: ContractRoleV1,
) -> Result<ContractRefV1, ProfileExecutionError> {
    let component = match role {
        ContractRoleV1::Affiliation => "reviewed-single-capture-roster-exact-utc-day-complete-connected-inventory-selector-with-gaps-and-conflicts-v1",
        ContractRoleV1::Publication => "selected-artifact8-frozen-graph-envelope-v3-reviewed-roster-descriptor-v2",
        _ => return selected_qualification_contract_ref_v3(role),
    };
    let digest = serialized_digest(&Descriptor {
        version: 1,
        revision: 5,
        role,
        scope: "retained-identity-graph-v3-source-lifecycle8-qualification-only",
        component,
    })?;
    Ok(ContractRefV1::new(1, 5, &digest)?)
}

pub fn validate_selected_qualification_profile_v4(
    profile: &QueryProfileV1,
) -> Result<SupportedQualificationProfileV4<'_>, ProfileExecutionError> {
    for role in ContractRoleV1::ALL {
        if role.reference(profile.input()) != &selected_qualification_contract_ref_v4(role)? {
            return Err(ProfileExecutionError::UnsupportedContractReference { role });
        }
    }
    check_selection_configuration(profile)?;
    Ok(SupportedQualificationProfileV4 { profile })
}

pub fn validate_current_selected_qualification_profile(
    profile: &QueryProfileV1,
) -> Result<(), ProfileExecutionError> {
    if profile.input().policies.affiliation
        == selected_qualification_contract_ref_v5(ContractRoleV1::Affiliation)?
    {
        validate_selected_qualification_profile_v5(profile).map(|_| ())
    } else if profile.input().policies.affiliation
        == selected_qualification_contract_ref_v4(ContractRoleV1::Affiliation)?
    {
        validate_selected_qualification_profile_v4(profile).map(|_| ())
    } else {
        super::validate_selected_qualification_profile_v3(profile).map(|_| ())
    }
}

pub fn validate_retained_selected_qualification_profile(
    profile: &QueryProfileV1,
    semantics: EventSemantics,
) -> Result<(), ProfileExecutionError> {
    let event = match semantics {
        EventSemantics::ResultGroups21 => {
            selected_qualification_contract_ref_v2(ContractRoleV1::Event)?
        }
        EventSemantics::ResultGroups22 => {
            selected_qualification_contract_ref_v3(ContractRoleV1::Event)?
        }
        _ => {
            return Err(ProfileExecutionError::UnsupportedContractReference {
                role: ContractRoleV1::Event,
            });
        }
    };
    let reference: fn(ContractRoleV1) -> Result<ContractRefV1, ProfileExecutionError> =
        if profile.input().policies.affiliation
            == selected_qualification_contract_ref_v5(ContractRoleV1::Affiliation)?
        {
            selected_qualification_contract_ref_v5
        } else if profile.input().policies.affiliation
            == selected_qualification_contract_ref_v4(ContractRoleV1::Affiliation)?
        {
            selected_qualification_contract_ref_v4
        } else {
            selected_qualification_contract_ref_v3
        };
    ContractRoleV1::ALL
        .into_iter()
        .try_for_each(|role| match role {
            ContractRoleV1::Event => require_retained_reference(profile, role, &event),
            _ => require_retained_reference(profile, role, &reference(role)?),
        })?;
    check_selection_configuration(profile)
}

fn require_retained_reference(
    profile: &QueryProfileV1,
    role: ContractRoleV1,
    expected: &ContractRefV1,
) -> Result<(), ProfileExecutionError> {
    if role.reference(profile.input()) != expected {
        return Err(ProfileExecutionError::UnsupportedContractReference { role });
    }
    Ok(())
}
