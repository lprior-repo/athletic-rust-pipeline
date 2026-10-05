use super::*;
use crate::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SourceRef {
    pub fn new(id: impl Into<String>, url: Option<String>) -> Self {
        Self { id: id.into(), url }
    }

    pub fn id(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            url: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMethod {
    Fetched,
    Parsed,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub source: SourceRef,
    pub method: EvidenceMethod,
    pub observed_on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Evidence {
    pub fn fetched(source: SourceRef, observed_on: impl Into<String>) -> Self {
        Self {
            source,
            method: EvidenceMethod::Fetched,
            observed_on: observed_on.into(),
            note: None,
        }
    }

    pub fn parsed(source: SourceRef, observed_on: impl Into<String>) -> Self {
        Self {
            source,
            method: EvidenceMethod::Parsed,
            observed_on: observed_on.into(),
            note: None,
        }
    }

    pub fn derived(
        source: SourceRef,
        observed_on: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            source,
            method: EvidenceMethod::Derived,
            observed_on: observed_on.into(),
            note: Some(note.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceNamespace {
    MilesplitSchool,
    MilesplitTeam,
    MilesplitAthlete,
    MilesplitMeet,
    TfrrsTeam,
    TfrrsAthlete,
    TfrrsMeet,
    DirectAthleticsTeam,
    DirectAthleticsAthlete,
    AssociationSchool {
        association: String,
    },
    SchoolDirectory {
        provider: String,
        state: UsJurisdiction,
    },
    AssociationAthlete {
        association: String,
    },
    TimerTeam {
        provider: String,
    },
    TimerAthlete {
        provider: String,
    },
    TimerMeet {
        provider: String,
    },
    LegacyAthleticNet {
        kind: String,
    },
    AthleticNet {
        kind: String,
    },
    Other(String),
}

impl SourceNamespace {
    pub fn is_core(&self) -> bool {
        !matches!(
            self,
            SourceNamespace::LegacyAthleticNet { .. } | SourceNamespace::AthleticNet { .. }
        )
    }

    pub fn association_school(association: &str) -> Self {
        Self::AssociationSchool {
            association: association.trim().to_ascii_lowercase(),
        }
    }

    pub fn school_directory(provider: &str, state: UsJurisdiction) -> Self {
        Self::SchoolDirectory {
            provider: provider.trim().to_ascii_lowercase(),
            state,
        }
    }

    pub fn athletic_net(kind: &str) -> Self {
        Self::AthleticNet {
            kind: kind.trim().to_ascii_lowercase(),
        }
    }
}

impl fmt::Display for SourceNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceNamespace::MilesplitSchool => f.write_str("milesplit_school"),
            SourceNamespace::MilesplitTeam => f.write_str("milesplit_team"),
            SourceNamespace::MilesplitAthlete => f.write_str("milesplit_athlete"),
            SourceNamespace::MilesplitMeet => f.write_str("milesplit_meet"),
            SourceNamespace::TfrrsTeam => f.write_str("tfrrs_team"),
            SourceNamespace::TfrrsAthlete => f.write_str("tfrrs_athlete"),
            SourceNamespace::TfrrsMeet => f.write_str("tfrrs_meet"),
            SourceNamespace::DirectAthleticsTeam => f.write_str("direct_athletics_team"),
            SourceNamespace::DirectAthleticsAthlete => f.write_str("direct_athletics_athlete"),
            SourceNamespace::AssociationSchool { association } => {
                write!(f, "association_school:{association}")
            }
            SourceNamespace::SchoolDirectory { provider, state } => {
                write!(f, "school_directory:{provider}:{}", state.code())
            }
            SourceNamespace::AssociationAthlete { association } => {
                write!(f, "association_athlete:{association}")
            }
            SourceNamespace::TimerTeam { provider } => write!(f, "timer_team:{provider}"),
            SourceNamespace::TimerAthlete { provider } => write!(f, "timer_athlete:{provider}"),
            SourceNamespace::TimerMeet { provider } => write!(f, "timer_meet:{provider}"),
            SourceNamespace::LegacyAthleticNet { kind } => {
                write!(f, "legacy_athletic_net:{kind}")
            }
            SourceNamespace::AthleticNet { kind } => write!(f, "athleticnet:{kind}"),
            SourceNamespace::Other(value) => f.write_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub namespace: SourceNamespace,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl SourceIdentity {
    pub fn new(namespace: SourceNamespace, id: impl Into<String>) -> Self {
        Self {
            namespace,
            id: id.into(),
            url: None,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8")]
pub struct Confidence(u8);

impl Confidence {
    pub const HIGH: Confidence = Confidence(85);
    pub const MEDIUM: Confidence = Confidence(65);
    pub const LOW: Confidence = Confidence(40);

    pub const fn new(value: u8) -> Option<Self> {
        if value > 100 {
            return None;
        }
        Some(Self(value))
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Confidence {
    type Error = ConfidenceError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(ConfidenceError::OutOfRange { value })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfidenceError {
    #[error("confidence {value} is outside the admitted 0..=100 range")]
    OutOfRange { value: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityStatus {
    Unverified,
    Verified,
    Pending,
    Rejected,
    RetainedConflict,
}

impl IdentityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Verified => "verified",
            Self::Pending => "pending",
            Self::Rejected => "rejected",
            Self::RetainedConflict => "retained_conflict",
        }
    }
}
