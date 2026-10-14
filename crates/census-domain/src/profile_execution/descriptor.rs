use super::policy::{policy_descriptor_v1, ProfilePolicyDescriptorV1};
use super::{ContractRoleV1, ProfileExecutionError};
use crate::model::{serialized_digest, ContractRefV1};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractDescriptorV1 {
    version: u32,
    revision: u32,
    role: ContractRoleV1,
    binding: ContractBindingV1,
    policy: ProfilePolicyDescriptorV1,
    runtime_execution: RuntimeExecutionV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContractBindingV1 {
    ConfigurationOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RuntimeExecutionV1 {
    NotCertified,
}

impl ContractDescriptorV1 {
    pub const fn role(&self) -> ContractRoleV1 {
        self.role
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn revision(&self) -> u32 {
        self.revision
    }
}

pub fn profile_contract_descriptor_v1(
    role: ContractRoleV1,
) -> Result<ContractDescriptorV1, ProfileExecutionError> {
    Ok(ContractDescriptorV1 {
        version: 1,
        revision: 1,
        role,
        binding: ContractBindingV1::ConfigurationOnly,
        policy: policy_descriptor_v1(role)?,
        runtime_execution: RuntimeExecutionV1::NotCertified,
    })
}

pub fn profile_contract_ref_v1(
    role: ContractRoleV1,
) -> Result<ContractRefV1, ProfileExecutionError> {
    let descriptor = profile_contract_descriptor_v1(role)?;
    let digest = serialized_digest(&descriptor)?;
    Ok(ContractRefV1::new(
        descriptor.version,
        descriptor.revision,
        &digest,
    )?)
}
