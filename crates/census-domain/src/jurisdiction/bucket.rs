use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::table::UsJurisdiction;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JurisdictionBucket {
    Jurisdiction(UsJurisdiction),
    #[default]
    Unplaced,
}

impl JurisdictionBucket {
    pub const UNPLACED_CODE: &'static str = "UNKNOWN";

    pub const fn code(self) -> &'static str {
        match self {
            Self::Jurisdiction(jurisdiction) => jurisdiction.code(),
            Self::Unplaced => Self::UNPLACED_CODE,
        }
    }

    pub const fn jurisdiction(self) -> Option<UsJurisdiction> {
        match self {
            Self::Jurisdiction(jurisdiction) => Some(jurisdiction),
            Self::Unplaced => None,
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        if code.trim().eq_ignore_ascii_case(Self::UNPLACED_CODE) {
            return Some(Self::Unplaced);
        }
        UsJurisdiction::from_code(code).map(Self::Jurisdiction)
    }
}

impl From<UsJurisdiction> for JurisdictionBucket {
    fn from(jurisdiction: UsJurisdiction) -> Self {
        Self::Jurisdiction(jurisdiction)
    }
}

impl From<Option<UsJurisdiction>> for JurisdictionBucket {
    fn from(jurisdiction: Option<UsJurisdiction>) -> Self {
        jurisdiction.map_or(Self::Unplaced, Self::Jurisdiction)
    }
}

impl fmt::Display for JurisdictionBucket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl Serialize for JurisdictionBucket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for JurisdictionBucket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_code(&raw).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "jurisdiction bucket {raw:?} is not one of the 50 states, the District of Columbia or {}",
                Self::UNPLACED_CODE
            ))
        })
    }
}
