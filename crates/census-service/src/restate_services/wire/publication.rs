use serde::{Deserialize, Serialize};

use super::NationalRequest;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationContract {
    #[default]
    LegacySeparateV1,
    ManagedBundleV2,
    SelectedPublicationV1,
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
}

impl PublicationCapabilities {
    pub fn current() -> Self {
        Self {
            publication_contracts: vec![PublicationContract::SelectedPublicationV1],
        }
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
