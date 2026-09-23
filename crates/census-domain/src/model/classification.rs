use super::*;

// -------------------------------------------------------------------------------------------------
// Classification
// -------------------------------------------------------------------------------------------------

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

    /// The byte spelling of this gender side inside a minted canonical id.
    ///
    /// An id is a persistence contract: the spelling is frozen here rather than taken from the
    /// derived `Debug`, because renaming a variant (or a change to how it is debug-printed) would
    /// otherwise re-mint every team, coach and event id already in the store. Changing a line below
    /// is then a visible edit to a key, not a side effect of a derive.
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
}

impl Sport {
    /// The byte spelling of this sport inside a minted canonical id; see [`Gender::stable_key`].
    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::OutdoorTrack => "OutdoorTrack",
            Self::IndoorTrack => "IndoorTrack",
            Self::CrossCountry => "CrossCountry",
        }
    }
}

/// A team is (school, sport, gender side, season).
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
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalTeam {
    /// One team per (school, sport, gender side, school year). The same four values always mint the
    /// same id, so a provider re-publishing a roster upserts the team instead of duplicating it.
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

/// Meet competition level, from our own vocabulary.
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

