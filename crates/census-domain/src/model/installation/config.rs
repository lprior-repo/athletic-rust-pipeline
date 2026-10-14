use super::{require_version, InstallationDomainError, ServiceResourceProfileV1};
use crate::model::{serialized_digest, ContractRefV1, ProfileId};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationCapabilityV1 {
    EnrollmentOnlyV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstallationConfigId(ProfileId);

impl InstallationConfigId {
    pub fn parse(value: &str) -> Result<Self, InstallationDomainError> {
        ProfileId::parse(value).map(Self).map_err(Into::into)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for InstallationConfigId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationConfigV1 {
    version: u32,
    initial_profile_id: ProfileId,
    service_resources: ServiceResourceProfileV1,
    source_registry: ContractRefV1,
    operating_defaults: ContractRefV1,
    capability: InstallationCapabilityV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: u32,
    initial_profile_id: ProfileId,
    service_resources: ServiceResourceProfileV1,
    source_registry: ContractRefV1,
    operating_defaults: ContractRefV1,
    capability: InstallationCapabilityV1,
}

impl InstallationConfigV1 {
    pub fn new(
        initial_profile_id: ProfileId,
        service_resources: ServiceResourceProfileV1,
        source_registry: ContractRefV1,
        operating_defaults: ContractRefV1,
        capability: InstallationCapabilityV1,
    ) -> Self {
        Self {
            version: 1,
            initial_profile_id,
            service_resources,
            source_registry,
            operating_defaults,
            capability,
        }
    }
    pub fn id(&self) -> Result<InstallationConfigId, InstallationDomainError> {
        InstallationConfigId::parse(&serialized_digest(self)?)
    }
    pub fn initial_profile_id(&self) -> &ProfileId {
        &self.initial_profile_id
    }
    pub const fn service_resources(&self) -> ServiceResourceProfileV1 {
        self.service_resources
    }
    pub fn source_registry(&self) -> &ContractRefV1 {
        &self.source_registry
    }
    pub fn operating_defaults(&self) -> &ContractRefV1 {
        &self.operating_defaults
    }
    pub const fn capability(&self) -> InstallationCapabilityV1 {
        self.capability
    }
}

impl<'de> Deserialize<'de> for InstallationConfigV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = Input::deserialize(deserializer)?;
        require_version("installation config", input.version).map_err(serde::de::Error::custom)?;
        Ok(Self::new(
            input.initial_profile_id,
            input.service_resources,
            input.source_registry,
            input.operating_defaults,
            input.capability,
        ))
    }
}
