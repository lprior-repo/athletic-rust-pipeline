use census_domain::model::{
    ATHLETE_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY, UNRESOLVED_VENUE_FAMILY,
};

pub const IDENTITY_FIELD: &str = "identity";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFamily {
    SchoolJurisdiction,
    MeetJurisdiction,
    AthleteIdentity,
}

impl ReviewFamily {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SchoolJurisdiction => UNRESOLVED_SCHOOL_FAMILY,
            Self::MeetJurisdiction => UNRESOLVED_VENUE_FAMILY,
            Self::AthleteIdentity => ATHLETE_IDENTITY_FAMILY,
        }
    }

    pub const fn field(self) -> &'static str {
        match self {
            Self::SchoolJurisdiction | Self::MeetJurisdiction => "state",
            Self::AthleteIdentity => IDENTITY_FIELD,
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        if label == UNRESOLVED_SCHOOL_FAMILY {
            Some(Self::SchoolJurisdiction)
        } else if label == UNRESOLVED_VENUE_FAMILY {
            Some(Self::MeetJurisdiction)
        } else if label == ATHLETE_IDENTITY_FAMILY {
            Some(Self::AthleteIdentity)
        } else {
            None
        }
    }

    pub const fn askable() -> [Self; 3] {
        [
            Self::SchoolJurisdiction,
            Self::MeetJurisdiction,
            Self::AthleteIdentity,
        ]
    }

    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        match normalized.as_str() {
            "school jurisdiction unresolved" | "school jurisdiction" | "school" => {
                Some(Self::SchoolJurisdiction)
            }
            "meet venue unresolved" | "meet venue" | "venue" | "meet jurisdiction" | "meet" => {
                Some(Self::MeetJurisdiction)
            }
            "athlete identity" | "athlete" | "identity" => Some(Self::AthleteIdentity),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOptions {
    pub families: Vec<ReviewFamily>,
    pub limit: usize,
    pub dry_run: bool,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            families: ReviewFamily::askable().to_vec(),
            limit: 25,
            dry_run: false,
        }
    }
}

#[cfg(test)]
#[path = "families_tests.rs"]
mod tests;
