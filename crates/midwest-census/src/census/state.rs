//! §48's completion lattice, and the seal that only its last phase may carry.
//!
//! Two rules are the whole point of this module:
//!
//! * **No phase skips.** A census walks `Discovering → Acquiring → Reconciling → Reviewing →
//!   ResolvingGaps → Exporting`, one step per [`CensusState::advance`]. `Acquiring → Complete` is
//!   not a slow path, it is a refused transition: a census with an empty HTTP queue is not a census
//!   whose work is terminal.
//! * **`Complete` carries a [`SealedCensus`], and only [`SealEvidence`] can produce one.** The
//!   evidence names every acceptance item §70 lists; [`SealEvidence::open_items`] returns the ones
//!   still unmet, so a refusal says *which* item blocked it rather than "not ready".
//!
//! Findings are not blockers. §70 asks for gaps, conflicts and exhausted retries to be *retained*,
//! not resolved, so a non-zero gap tally seals fine and stays visible inside the seal — what
//! refuses a seal is an unfinished decision, an unreconciled workbook, or a count the store cannot
//! reproduce.

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
    JurisdictionStages, SourceObject,
};

/// The phases §48 names, in the only order they may be entered.
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
    /// Every phase, in lattice order.
    pub const ALL: [Phase; 7] = [
        Phase::Discovering,
        Phase::Acquiring,
        Phase::Reconciling,
        Phase::Reviewing,
        Phase::ResolvingGaps,
        Phase::Exporting,
        Phase::Complete,
    ];

    /// The name a report prints, which is also the name it serializes to.
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

    /// Position in the lattice, so a comparison reads as "is this later than that".
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

    /// The one phase this phase may advance to; `None` for [`Phase::Complete`].
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

    /// The state a phase *entered by an advance* produces: every phase but [`Phase::Complete`],
    /// which is reached through a seal rather than through a bare state.
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

/// Where one census stands. `Complete` is unconstructible without a seal.
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
    Complete(SealedCensus),
}

impl CensusState {
    /// The phase this state is in.
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

    /// The seal, when this census has one.
    pub const fn sealed(&self) -> Option<&SealedCensus> {
        match self {
            CensusState::Complete(sealed) => Some(sealed),
            _ => None,
        }
    }

    /// Move one step along the lattice.
    ///
    /// [`Phase::Complete`] is not a legal target here even from [`Phase::Exporting`]: completion is
    /// [`CensusState::seal`], which demands evidence. Everything else must be the immediate next
    /// phase, so a resumed run cannot jump the phases it has not run.
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

    /// Seal the census, or refuse and name the item that blocked it.
    ///
    /// A state that is already sealed is left exactly as it is: sealing twice must not mint a second
    /// digest for the same census.
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
        *self = CensusState::Complete(evidence.into_seal());
        Ok(())
    }
}

/// Why a transition or a seal was refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SealError {
    /// The transition is not the next phase, or completion was requested as a bare advance.
    #[error("census cannot move from {from} to {to}: each phase advances one step, and completion needs a seal")]
    OutOfOrder {
        from: &'static str,
        to: &'static str,
    },
    /// A state that is neither exporting nor sealed was asked to complete.
    #[error("census in {from} cannot complete: {detail}")]
    Unsealed { from: &'static str, detail: String },
    /// An acceptance item of §70 is unmet.
    #[error("census cannot be sealed: {} is unmet ({detail})", item.as_str())]
    ItemUnmet {
        item: AcceptanceItem,
        detail: String,
    },
}
