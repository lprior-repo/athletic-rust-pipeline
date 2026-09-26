use super::*;

/// A registered data source. `id` is a stable slug used in evidence and reports.
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

/// How a fact was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMethod {
    /// Read directly from a fetched document (HTML/JSON/PDF/CSV).
    Fetched,
    /// Extracted by parsing a fetched document.
    Parsed,
    /// Deterministically derived from other evidence (e.g. grade + school year -> grad year).
    Derived,
    /// Asserted by an upstream dataset that is itself under evidence (legacy import).
    Inherited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub source: SourceRef,
    pub method: EvidenceMethod,
    /// ISO-8601 date the source was observed.
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

/// Namespace of an external identity. Namespaces are open-ended by design (`Other("...")`) so new
/// providers never force a model change; the well-known ones are enumerated for type safety.
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
    /// `association` is the state association slug (`wiaa`, `ihsa`, `kshsaa`, …).
    AssociationSchool {
        association: String,
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
    /// Historical Athletic.net-derived ids retained as legacy evidence only.
    LegacyAthleticNet {
        kind: String,
    },
    /// Athletic.net ids read through the owner-authorized athlete-bio adapter (`athlete`, `school`,
    /// `meet`). Non-core: it is the same vendor the platform's own adapters exist to be
    /// independent of.
    AthleticNet {
        kind: String,
    },
    Other(String),
}

impl SourceNamespace {
    /// True when the namespace is supplied by one of the platform's own adapters rather than by
    /// Athletic.net or its mirror.
    ///
    /// Only the Athletic.net namespaces are non-core by definition. Timer namespaces stay core: the
    /// AthleticLIVE-derived rows that carry them are already excluded by their evidence source id,
    /// while a real timing provider (`pttiming`, `wayzata`, …) is a core source.
    pub fn is_core(&self) -> bool {
        !matches!(
            self,
            SourceNamespace::LegacyAthleticNet { .. } | SourceNamespace::AthleticNet { .. }
        )
    }

    /// The namespace a state association's own directory pages file a school under.
    ///
    /// One constructor so every adapter that reads an association's members names it the same way:
    /// two spellings of one association would split its schools across two unrelated namespaces.
    pub fn association_school(association: &str) -> Self {
        Self::AssociationSchool {
            association: association.trim().to_ascii_lowercase(),
        }
    }

    /// The namespace an Athletic.net page kind files an object under (`school`, `team`, …).
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

/// An identity this entity carries in some external system.
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

/// Confidence in an identity merge or field value, 0..=100.
///
/// The percentage is private so a caller cannot store a value the scale has no room for: [`Self::new`]
/// is the only way in from a bare number, and a mis-scaled input (a 0..1 ratio, a 0..1000 score)
/// fails there instead of reading as full confidence forever after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(u8);

impl Confidence {
    pub const HIGH: Confidence = Confidence(85);
    pub const MEDIUM: Confidence = Confidence(65);
    pub const LOW: Confidence = Confidence(40);

    /// The confidence a caller measured; `None` above 100, because there is no such confidence.
    pub const fn new(value: u8) -> Option<Self> {
        if value > 100 {
            return None;
        }
        Some(Self(value))
    }

    /// The percentage as stored.
    pub const fn get(self) -> u8 {
        self.0
    }
}
