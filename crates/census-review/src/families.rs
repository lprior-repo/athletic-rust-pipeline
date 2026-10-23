use census_domain::model::{
    ATHLETE_GENDER_UNVERIFIED_FAMILY, ATHLETE_IDENTITY_FAMILY, SCHOOL_ADDRESS_UNVERIFIED_FAMILY,
    SCHOOL_IDENTITY_FAMILY, TRANSFER_AMBIGUOUS_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY,
};

pub const IDENTITY_FIELD: &str = "identity";
pub const GENDER_FIELD: &str = "gender";
pub const SCHOOL_ADDRESS_FIELD: &str = "school_address";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFamily {
    SchoolJurisdiction,
    MeetJurisdiction,
    AthleteIdentity,
    SchoolLink,
    AthleteGender,
    SchoolAddress,
    TransferAmbiguous,
}

impl ReviewFamily {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SchoolJurisdiction => UNRESOLVED_SCHOOL_FAMILY,
            Self::MeetJurisdiction => UNRESOLVED_VENUE_FAMILY,
            Self::AthleteIdentity => ATHLETE_IDENTITY_FAMILY,
            Self::SchoolLink => SCHOOL_IDENTITY_FAMILY,
            Self::AthleteGender => ATHLETE_GENDER_UNVERIFIED_FAMILY,
            Self::SchoolAddress => SCHOOL_ADDRESS_UNVERIFIED_FAMILY,
            Self::TransferAmbiguous => TRANSFER_AMBIGUOUS_FAMILY,
        }
    }

    pub const fn field(self) -> &'static str {
        match self {
            Self::SchoolJurisdiction | Self::MeetJurisdiction => "state",
            Self::AthleteIdentity | Self::SchoolLink => IDENTITY_FIELD,
            Self::AthleteGender => GENDER_FIELD,
            Self::SchoolAddress => SCHOOL_ADDRESS_FIELD,
            Self::TransferAmbiguous => IDENTITY_FIELD,
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        if label == UNRESOLVED_SCHOOL_FAMILY {
            Some(Self::SchoolJurisdiction)
        } else if label == UNRESOLVED_VENUE_FAMILY {
            Some(Self::MeetJurisdiction)
        } else if label == ATHLETE_IDENTITY_FAMILY {
            Some(Self::AthleteIdentity)
        } else if label == SCHOOL_IDENTITY_FAMILY {
            Some(Self::SchoolLink)
        } else if label == ATHLETE_GENDER_UNVERIFIED_FAMILY {
            Some(Self::AthleteGender)
        } else if label == SCHOOL_ADDRESS_UNVERIFIED_FAMILY {
            Some(Self::SchoolAddress)
        } else if label == TRANSFER_AMBIGUOUS_FAMILY {
            Some(Self::TransferAmbiguous)
        } else {
            None
        }
    }

    pub const fn askable() -> [Self; 5] {
        [
            Self::SchoolJurisdiction,
            Self::MeetJurisdiction,
            Self::AthleteIdentity,
            Self::AthleteGender,
            Self::SchoolAddress,
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
            "school identity" | "school link" => Some(Self::SchoolLink),
            "athlete gender unverified" | "athlete gender" | "gender" => Some(Self::AthleteGender),
            "school address unverified" | "school address" | "address" => Some(Self::SchoolAddress),
            "transfer ambiguous" | "transfer" => Some(Self::TransferAmbiguous),
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
