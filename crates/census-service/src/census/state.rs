
use serde::{Deserialize, Serialize};

mod evidence;
mod open;
mod seal_digest;
#[cfg(test)]
mod tests;

pub use evidence::{
    AcceptanceItem, GapTally, OpenWork, RetainedFindings, SealCounts, SealEvidence, SealedCensus,
    WorkbookCheck,
};
pub use open::{
    owed_cohort_decisions, owed_identity_candidates, owed_jurisdictions, owed_source_objects,
    silent_source_objects, JurisdictionStages, SourceObject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Discovering,
    Acquiring,
    Reconciling,
    Reviewing,
    ResolvingGaps,
    Exporting,
    Complete,
}

impl Phase {
    pub const ALL: [Phase; 7] = [
        Phase::Discovering,
        Phase::Acquiring,
        Phase::Reconciling,
        Phase::Reviewing,
        Phase::ResolvingGaps,
        Phase::Exporting,
        Phase::Complete,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Phase::Discovering => "discovering",
            Phase::Acquiring => "acquiring",
            Phase::Reconciling => "reconciling",
            Phase::Reviewing => "reviewing",
            Phase::ResolvingGaps => "resolving_gaps",
            Phase::Exporting => "exporting",
            Phase::Complete => "complete",
        }
    }

    pub const fn ordinal(self) -> u8 {
        match self {
            Phase::Discovering => 0,
            Phase::Acquiring => 1,
            Phase::Reconciling => 2,
            Phase::Reviewing => 3,
            Phase::ResolvingGaps => 4,
            Phase::Exporting => 5,
            Phase::Complete => 6,
        }
    }

    pub const fn next(self) -> Option<Phase> {
        match self {
            Phase::Discovering => Some(Phase::Acquiring),
            Phase::Acquiring => Some(Phase::Reconciling),
            Phase::Reconciling => Some(Phase::Reviewing),
            Phase::Reviewing => Some(Phase::ResolvingGaps),
            Phase::ResolvingGaps => Some(Phase::Exporting),
            Phase::Exporting | Phase::Complete => None,
        }
    }

    const fn open_state(self) -> Option<CensusState> {
        match self {
            Phase::Discovering => Some(CensusState::Discovering),
            Phase::Acquiring => Some(CensusState::Acquiring),
            Phase::Reconciling => Some(CensusState::Reconciling),
            Phase::Reviewing => Some(CensusState::Reviewing),
            Phase::ResolvingGaps => Some(CensusState::ResolvingGaps),
            Phase::Exporting => Some(CensusState::Exporting),
            Phase::Complete => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum CensusState {
    #[default]
    Discovering,
    Acquiring,
    Reconciling,
    Reviewing,
    ResolvingGaps,
    Exporting,
    Complete(Box<SealedCensus>),
}

impl CensusState {
    pub const fn phase(&self) -> Phase {
        match self {
            CensusState::Discovering => Phase::Discovering,
            CensusState::Acquiring => Phase::Acquiring,
            CensusState::Reconciling => Phase::Reconciling,
            CensusState::Reviewing => Phase::Reviewing,
            CensusState::ResolvingGaps => Phase::ResolvingGaps,
            CensusState::Exporting => Phase::Exporting,
            CensusState::Complete(_) => Phase::Complete,
        }
    }

    pub const fn sealed(&self) -> Option<&SealedCensus> {
        match self {
            CensusState::Complete(sealed) => Some(&**sealed),
            _ => None,
        }
    }

    pub fn advance(&mut self, to: Phase) -> Result<(), SealError> {
        let from = self.phase();
        if to == Phase::Complete || from.next() != Some(to) {
            return Err(SealError::OutOfOrder {
                from: from.as_str(),
                to: to.as_str(),
            });
        }
        let entered = to.open_state().ok_or(SealError::Unsealed {
            from: from.as_str(),
            detail: "completion requires SealEvidence; use seal()".to_string(),
        })?;
        *self = entered;
        Ok(())
    }

    pub fn seal(&mut self, evidence: SealEvidence) -> Result<(), SealError> {
        if self.sealed().is_some() {
            return Ok(());
        }
        if self.phase() != Phase::Exporting {
            return Err(SealError::OutOfOrder {
                from: self.phase().as_str(),
                to: Phase::Complete.as_str(),
            });
        }
        if let Some(item) = evidence.open_items().first().copied() {
            return Err(SealError::ItemUnmet {
                item,
                detail: evidence.detail(item),
            });
        }
        *self = CensusState::Complete(Box::new(evidence.into_seal()));
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SealError {
    #[error("census cannot move from {from} to {to}: each phase advances one step, and completion needs a seal")]
    OutOfOrder {
        from: &'static str,
        to: &'static str,
    },
    #[error("census in {from} cannot complete: {detail}")]
    Unsealed { from: &'static str, detail: String },
    #[error("census cannot be sealed: {} is unmet ({detail})", item.as_str())]
    ItemUnmet {
        item: AcceptanceItem,
        detail: String,
    },
}
