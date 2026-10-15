mod binding;
mod budget;
mod codec;
mod error;
mod evidence;
mod graph;
pub mod local_adjudication;
mod matched_group;
mod merge;
mod record;
mod replay;
mod request_order;
mod resolution;
mod revocation;
mod split;
mod types;

pub use codec::{CandidateHistorySeed, DecodedCandidateHistory};
pub use error::HistoryError;
pub use evidence::{CandidateDecisionReference, CandidateEvidenceKind};
pub use record::{genesis, prepare_next, HistoryRecord, PreparedHistoryRecord, RegistryHead};
pub use replay::{replay_candidate_history, HistoryProjection};
pub use types::{
    BindingClaim, BindingPartition, BindingResolution, EffectiveScope, EstablishmentOrdinal,
    HistoryRequest, PublicAthleteId, PublicResolution, ReplayLimits, ResolvedSuccessor,
    RevocationRequest, SourceObjectKey, SplitRequest,
};

#[cfg(test)]
mod tests;
