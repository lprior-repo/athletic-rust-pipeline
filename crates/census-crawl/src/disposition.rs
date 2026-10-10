use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionDisposition {
    #[default]
    #[serde(alias = "Unknown")]
    Unknown,
    #[serde(alias = "Complete")]
    Complete,
    #[serde(alias = "Partial")]
    Partial,
    #[serde(alias = "Blocked")]
    Blocked,
    #[serde(alias = "Failed")]
    Failed,
    #[serde(alias = "Quarantined")]
    Quarantined,
    #[serde(alias = "Exhausted")]
    Exhausted,
}

impl CollectionDisposition {
    pub fn is_complete(self) -> bool {
        self == Self::Complete
    }
}
