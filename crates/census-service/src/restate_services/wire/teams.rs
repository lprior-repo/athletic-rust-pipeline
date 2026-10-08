use serde::{Deserialize, Serialize};

use super::{StageOutcome, TeamsSourceFailure};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "status", content = "outcome", rename_all = "snake_case")]
pub enum TeamsStage {
    #[default]
    Owed,
    Completed(CompletedTeams),
    Failed(TeamsFailure),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "StageOutcome")]
pub struct CompletedTeams(StageOutcome);

#[derive(Debug, Clone, thiserror::Error)]
#[error("a teams stage with retained source errors cannot be completed")]
pub struct IncompleteTeams(StageOutcome);

impl TryFrom<StageOutcome> for CompletedTeams {
    type Error = IncompleteTeams;

    fn try_from(outcome: StageOutcome) -> Result<Self, Self::Error> {
        if outcome.disposition.is_complete()
            && outcome.errors.is_empty()
            && outcome.unfinished.is_empty()
        {
            Ok(Self(outcome))
        } else {
            Err(IncompleteTeams(outcome))
        }
    }
}

impl CompletedTeams {
    pub fn outcome(&self) -> &StageOutcome {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TeamsFailure {
    ActionTerminal {
        at: String,
        code: u16,
        message: String,
    },
    IncompleteOutcome {
        at: String,
        outcome: StageOutcome,
    },
    SourceFailures {
        at: String,
        outcome: StageOutcome,
        failures: Vec<TeamsSourceFailure>,
    },
}

impl TeamsStage {
    pub fn is_owed(&self) -> bool {
        matches!(self, Self::Owed)
    }

    pub fn is_resumable(&self) -> bool {
        match self {
            Self::Owed => true,
            Self::Completed(_) => false,
            Self::Failed(failure) => match failure {
                TeamsFailure::ActionTerminal { .. } => false,
                TeamsFailure::IncompleteOutcome { .. } | TeamsFailure::SourceFailures { .. } => {
                    true
                }
            },
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }

    pub(crate) fn from_outcome(outcome: StageOutcome, at: String) -> Self {
        match CompletedTeams::try_from(outcome) {
            Ok(completed) => Self::Completed(completed),
            Err(IncompleteTeams(outcome)) => {
                Self::Failed(TeamsFailure::IncompleteOutcome { at, outcome })
            }
        }
    }
}
