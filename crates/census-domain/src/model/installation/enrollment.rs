use super::{require_version, InstallationConfigId};
use crate::model::StoreIdentity;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationEnrollmentV1 {
    version: u32,
    store_identity: StoreIdentity,
    config_id: InstallationConfigId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    version: u32,
    store_identity: StoreIdentity,
    config_id: InstallationConfigId,
}

impl InstallationEnrollmentV1 {
    pub fn new(store_identity: StoreIdentity, config_id: InstallationConfigId) -> Self {
        Self {
            version: 1,
            store_identity,
            config_id,
        }
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn config_id(&self) -> &InstallationConfigId {
        &self.config_id
    }
}

impl<'de> Deserialize<'de> for InstallationEnrollmentV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = Input::deserialize(deserializer)?;
        require_version("installation enrollment", input.version)
            .map_err(serde::de::Error::custom)?;
        Ok(Self::new(input.store_identity, input.config_id))
    }
}
