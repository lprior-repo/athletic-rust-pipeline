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
}

impl Scenario {
    pub const ALL: [Self; 10] = [
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
        }
    }

    pub fn all_names() -> Vec<String> {
        Self::ALL
            .into_iter()
            .map(Self::as_str)
            .map(str::to_owned)
            .collect()
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
