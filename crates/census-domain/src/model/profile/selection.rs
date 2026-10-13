use super::ProfileId;
use crate::model::StoreIdentity;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileSelectionError {
    #[error("unsupported profile-selection version {found}")]
    UnsupportedVersion { found: u32 },
    #[error("profile selection belongs to a different store identity")]
    ForeignStore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileSelectionV1 {
    version: u32,
    store_identity: StoreIdentity,
    profile_id: ProfileId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionInput {
    version: u32,
    store_identity: StoreIdentity,
    profile_id: ProfileId,
}

impl ProfileSelectionV1 {
    pub fn new(store_identity: StoreIdentity, profile_id: ProfileId) -> Self {
        Self {
            version: 1,
            store_identity,
            profile_id,
        }
    }

    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }

    pub fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }

    pub fn ensure_store(&self, actual: &StoreIdentity) -> Result<(), ProfileSelectionError> {
        if &self.store_identity != actual {
            return Err(ProfileSelectionError::ForeignStore);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for ProfileSelectionV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = SelectionInput::deserialize(deserializer)?;
        if input.version != 1 {
            return Err(serde::de::Error::custom(
                ProfileSelectionError::UnsupportedVersion {
                    found: input.version,
                },
            ));
        }
        Ok(Self::new(input.store_identity, input.profile_id))
    }
}
