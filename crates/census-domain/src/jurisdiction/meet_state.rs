
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::table::UsJurisdiction;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeetState {
    Placed(UsJurisdiction),
    #[default]
    Unresolved,
}

impl MeetState {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Placed(jurisdiction) => jurisdiction.code(),
            Self::Unresolved => crate::model::MEET_STATE_UNRESOLVED,
        }
    }

    pub const fn jurisdiction(self) -> Option<UsJurisdiction> {
        match self {
            Self::Placed(jurisdiction) => Some(jurisdiction),
            Self::Unresolved => None,
        }
    }
}

impl From<UsJurisdiction> for MeetState {
    fn from(jurisdiction: UsJurisdiction) -> Self {
        Self::Placed(jurisdiction)
    }
}

impl From<Option<UsJurisdiction>> for MeetState {
    fn from(jurisdiction: Option<UsJurisdiction>) -> Self {
        jurisdiction.map_or(Self::Unresolved, Self::Placed)
    }
}

impl fmt::Display for MeetState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl Serialize for MeetState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for MeetState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        if raw.trim() == crate::model::MEET_STATE_UNRESOLVED {
            return Ok(Self::Unresolved);
        }
        UsJurisdiction::from_code(&raw).map(Self::Placed).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "meet state {raw:?} is neither {} nor one of the 50 states or the District of Columbia",
                crate::model::MEET_STATE_UNRESOLVED
            ))
        })
    }
}
