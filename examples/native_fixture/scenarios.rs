use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    Match,
    Duplicate,
    Ambiguous,
    MissingCohort,
    Conflict,
    EmptySearch,
    Malformed,
    RetryExhaustion,
    PayloadLimit,
    AccessDenied,
    SplitLocation,
    GenericSchool,
    NameExclusion,
    ProbeFailure,
    BioIdentityConflict,
    HtmlIdentityUnknown,
    IncompleteIdentity,
    WrongBioId,
    MisleadingSearchName,
    MissingHtmlHint,
    RawIdentityConflict,
    HtmlAliasConflict,
}

impl Scenario {
    pub const ALL: [Self; 22] = [
        Self::Match,
        Self::Duplicate,
        Self::Ambiguous,
        Self::MissingCohort,
        Self::Conflict,
        Self::EmptySearch,
        Self::Malformed,
        Self::RetryExhaustion,
        Self::PayloadLimit,
        Self::AccessDenied,
        Self::SplitLocation,
        Self::GenericSchool,
        Self::NameExclusion,
        Self::ProbeFailure,
        Self::BioIdentityConflict,
        Self::HtmlIdentityUnknown,
        Self::IncompleteIdentity,
        Self::WrongBioId,
        Self::MisleadingSearchName,
        Self::MissingHtmlHint,
        Self::RawIdentityConflict,
        Self::HtmlAliasConflict,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Duplicate => "duplicate",
            Self::Ambiguous => "ambiguous",
            Self::MissingCohort => "missing-cohort",
            Self::Conflict => "conflict",
            Self::EmptySearch => "empty-search",
            Self::Malformed => "malformed",
            Self::RetryExhaustion => "retry-exhaustion",
            Self::PayloadLimit => "payload-limit",
            Self::AccessDenied => "access-denied",
            Self::SplitLocation => "split-location",
            Self::GenericSchool => "generic-school",
            Self::NameExclusion => "name-exclusion",
            Self::ProbeFailure => "probe-failure",
            Self::BioIdentityConflict => "bio-identity-conflict",
            Self::HtmlIdentityUnknown => "html-identity-unknown",
            Self::IncompleteIdentity => "incomplete-identity",
            Self::WrongBioId => "wrong-bio-id",
            Self::MisleadingSearchName => "misleading-search-name",
            Self::MissingHtmlHint => "missing-html-hint",
            Self::RawIdentityConflict => "raw-identity-conflict",
            Self::HtmlAliasConflict => "html-alias-conflict",
        }
    }

    pub fn all_names() -> Vec<String> {
        Self::ALL
            .into_iter()
            .map(Self::as_str)
            .map(str::to_owned)
            .collect()
    }

    pub fn identity_query(normalized: &str) -> Option<Self> {
        const CASES: [(&str, Scenario); 15] = [
            ("coverage", Scenario::NameExclusion),
            ("cover'age", Scenario::NameExclusion),
            ("cover age", Scenario::NameExclusion),
            ("other", Scenario::NameExclusion),
            ("exclusion", Scenario::NameExclusion),
            ("failurecase", Scenario::ProbeFailure),
            ("bioconflict", Scenario::BioIdentityConflict),
            ("statecase", Scenario::HtmlIdentityUnknown),
            ("componentcase", Scenario::IncompleteIdentity),
            ("bindingcase", Scenario::WrongBioId),
            ("displaycase", Scenario::MisleadingSearchName),
            ("nohintcase", Scenario::MissingHtmlHint),
            ("rawcase", Scenario::RawIdentityConflict),
            ("htmlaliascase", Scenario::HtmlAliasConflict),
            ("cover’age", Scenario::NameExclusion),
        ];
        CASES
            .iter()
            .find_map(|(needle, case)| normalized.contains(needle).then_some(*case))
    }
}

impl FromStr for Scenario {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|scenario| scenario.as_str() == value)
            .ok_or_else(|| {
                format!(
                    "unknown fixture scenario {value:?}; choose from {}",
                    Self::all_names().join(", ")
                )
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSet {
    pub scenarios: Vec<Scenario>,
}

impl ScenarioSet {
    pub fn all() -> Self {
        Self {
            scenarios: Scenario::ALL.to_vec(),
        }
    }

    pub fn contains(&self, scenario: Scenario) -> bool {
        self.scenarios.contains(&scenario)
    }
}
