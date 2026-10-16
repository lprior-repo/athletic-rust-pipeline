mod binding;
mod budget;
mod codec;
mod error;
mod evidence;
mod graph;
mod graph_reference;
pub use graph_reference::GraphAuthorityReferenceV1;
pub mod local_adjudication;
pub mod local_membership_withholding;
mod matched_group;
mod membership;
mod merge;
mod record;
mod replay;
mod request_order;
mod resolution;
mod revocation;
mod split;
mod types;
mod withholding;

pub use codec::{CandidateHistorySeed, DecodedCandidateHistory};
pub use error::HistoryError;
pub use evidence::{CandidateDecisionReference, CandidateEvidenceKind};
pub use record::{genesis, prepare_next, HistoryRecord, PreparedHistoryRecord, RegistryHead};
pub use replay::{replay_candidate_history, HistoryProjection};
pub use types::{
    BindingClaim, BindingPartition, BindingResolution, CandidateMembershipClaimV1, EffectiveScope,
    EstablishmentOrdinal, HistoryRequest, PublicAthleteId, PublicResolution, ReplayLimits,
    ResolvedSuccessor, RevocationRequest, SourceObjectKey, SplitRequest,
};

#[cfg(test)]
mod tests;
