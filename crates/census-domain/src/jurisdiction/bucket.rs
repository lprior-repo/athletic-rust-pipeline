//! The `JurisdictionBucket` enum and all its implementations.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use super::table::UsJurisdiction;

/// The jurisdiction a report or workbook row publishes in: a covered jurisdiction, or the one row
/// that holds everything no evidence placed.
///
/// The report's per-state buckets are keyed by this rather than by a printed code, so a row's
/// jurisdiction stays a validated value from the store to the cell and is rendered — by [`code`] or
/// [`Display`] — only when it is written out. `Unplaced` is declared last so the derived `Ord` puts
/// the unplaced row after every jurisdiction, which is the order the coverage report documents for
/// its own rows; writers that must reproduce a published row order sort with an explicit key.
///
/// [`code`]: Self::code
/// [`Display`]: fmt::Display
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum JurisdictionBucket {
    /// A school or meet row placed in one of the fifty states or the District of Columbia.
    Jurisdiction(UsJurisdiction),
    /// No evidence placed the row: a school whose own `state` is `None`, or a meet that never says
    /// where it was held. The census keeps these entities and reports them; it does not invent a
    /// jurisdiction for them.
    ///
    /// This is also the [`Default`], because a report row that has not been placed must read as
    /// *unplaced* rather than as some jurisdiction: the default can lose a label, never fabricate
    /// one.
    #[default]
    Unplaced,
}

impl JurisdictionBucket {
    /// The label printed for everything no evidence placed — the label every report and workbook
    /// this program has published already uses.
    pub const UNPLACED_CODE: &'static str = "UNKNOWN";

    /// The label this bucket prints as: the USPS code, or [`Self::UNPLACED_CODE`].
    pub const fn code(self) -> &'static str {
        match self {
            Self::Jurisdiction(jurisdiction) => jurisdiction.code(),
            Self::Unplaced => Self::UNPLACED_CODE,
        }
    }

    /// The placed jurisdiction, or `None` for the unplaced row.
    pub const fn jurisdiction(self) -> Option<UsJurisdiction> {
        match self {
            Self::Jurisdiction(jurisdiction) => Some(jurisdiction),
            Self::Unplaced => None,
        }
    }

    /// Parse a printed label: [`Self::UNPLACED_CODE`] or any USPS code this census covers.
    ///
    /// Case and surrounding whitespace are ignored, and territories still fail — the same rule the
    /// jurisdiction itself follows, because a report row is a coverage claim.
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
    /// `Some` is the placed jurisdiction; `None` is the unplaced row, never a dropped one.
    fn from(jurisdiction: Option<UsJurisdiction>) -> Self {
        jurisdiction.map_or(Self::Unplaced, Self::Jurisdiction)
    }
}

impl fmt::Display for JurisdictionBucket {
    /// Displays [`Self::code`], the form every published row carries.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl Serialize for JurisdictionBucket {
    /// Serializes as the printed label (`"WI"`, `"UNKNOWN"`), so a bucket is a valid JSON map key
    /// and every published document keeps the shape its readers already parse.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for JurisdictionBucket {
    /// An unknown label is a decode error rather than a retained string — the state a bucket names
    /// is either covered or explicitly unplaced.
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
