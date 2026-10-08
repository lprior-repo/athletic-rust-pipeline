use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionDisposition {
    #[default]
    Unknown,
    Complete,
    Partial,
    Blocked,
    Failed,
    Quarantined,
    Exhausted,
}

impl CollectionDisposition {
    pub fn is_complete(self) -> bool {
        self == Self::Complete
    }
}
