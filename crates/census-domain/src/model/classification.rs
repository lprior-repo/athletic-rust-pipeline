use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Boys,
    Girls,
    Mixed,
    Unknown,
}

impl Gender {
    pub fn parse_milesplit(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "m" | "male" | "boys" | "boy" => Gender::Boys,
            "f" | "female" | "girls" | "girl" => Gender::Girls,
            _ => Gender::Unknown,
        }
    }

    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::Boys => "Boys",
            Self::Girls => "Girls",
            Self::Mixed => "Mixed",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sport {
    OutdoorTrack,
    IndoorTrack,
    CrossCountry,
    Unknown,
}

impl Sport {
    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::OutdoorTrack => "OutdoorTrack",
            Self::IndoorTrack => "IndoorTrack",
            Self::CrossCountry => "CrossCountry",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalTeam {
    pub id: TeamId,
    pub school: SchoolId,
    pub sport: Sport,
    pub gender: Gender,
    pub school_year: SchoolYear,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalTeam {
    pub fn mint(
        school: &SchoolId,
        sport: Sport,
        gender: Gender,
        school_year: SchoolYear,
    ) -> TeamId {
        Id::mint(
            "team",
            &[
                school.as_str(),
                sport.stable_key(),
                gender.stable_key(),
                &school_year.get().to_string(),
            ],
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionLevel {
    Invitational,
    Dual,
    Conference,
    District,
    Regional,
    Sectional,
    State,
    National,
    Unknown,
}
