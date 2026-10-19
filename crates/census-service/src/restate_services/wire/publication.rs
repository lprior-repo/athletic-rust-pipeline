use serde::{Deserialize, Serialize};

use super::NationalRequest;

const NATIONAL_GENERATION_REVISION: u16 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationContract {
    #[default]
    LegacySeparateV1,
    ManagedBundleV2,
    SelectedPublicationV1,
    SelectedPublicationV2,
}

impl PublicationContract {
    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::LegacySeparateV1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationCapabilities {
    pub publication_contracts: Vec<PublicationContract>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub national_generation_revision: Option<u16>,
}

impl PublicationCapabilities {
    pub fn current() -> Self {
        Self {
            publication_contracts: vec![
                PublicationContract::SelectedPublicationV1,
                PublicationContract::SelectedPublicationV2,
            ],
            national_generation_revision: Some(NATIONAL_GENERATION_REVISION),
        }
    }

    pub fn supports_national_generation(&self) -> bool {
        self.national_generation_revision == Some(NATIONAL_GENERATION_REVISION)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NationalRequestBinding {
    Bound(Box<NationalRequest>),
    PendingPreflight,
    UnboundLegacy,
}

#[cfg(test)]
mod tests;
