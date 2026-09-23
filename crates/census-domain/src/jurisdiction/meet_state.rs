//! The `MeetState` enum and all its implementations.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::table::UsJurisdiction;

/// The state a *meet* publishes in: the jurisdiction the venue sits in, or the store's unresolved
/// sentinel when the meet never says.
///
/// This is deliberately a different type from [`super::bucket::JurisdictionBucket`]. A school that names no state
/// publishes in the unplaced row (`UNKNOWN`), because a school without a state is a missing fact
/// about the school universe; a meet that names no venue state publishes the sentinel the store has
/// always written (`??`), because most meets simply do not carry one and the label is how every
/// published artifact already reads. Typing it keeps the choice at the boundary instead of in a
/// string a caller could invent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MeetState {
    /// The venue's jurisdiction, as the meet row stated it.
    Placed(UsJurisdiction),
    /// The meet names no place. The default, so a meet never silently claims a state.
    #[default]
    Unresolved,
}

impl MeetState {
    /// The label this state prints as: the USPS code, or [`crate::model::MEET_STATE_UNRESOLVED`].
    pub const fn code(self) -> &'static str {
        match self {
            Self::Placed(jurisdiction) => jurisdiction.code(),
            Self::Unresolved => crate::model::MEET_STATE_UNRESOLVED,
        }
    }

    /// The placed jurisdiction, or `None` for a meet that never states one.
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
    /// Displays [`Self::code`], the form every published row carries — sentinel included.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl Serialize for MeetState {
    /// Serializes as the printed label (`"WI"` / `"??"`), so the best-result sidecars, the meet
    /// inventory and the coverage JSON keep the bytes their readers already parse.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for MeetState {
    /// The sentinel decodes back to *unresolved*; any other label must be a covered jurisdiction.
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
