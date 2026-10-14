use crate::model::{AthleteCandidateId, ProfileDateRangeV1, SourceNamespace};
use serde::Serialize;

use super::{CandidateDecisionReference, HistoryError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PublicAthleteId(pub(super) String);

impl PublicAthleteId {
    pub fn parse(value: &str) -> Result<Self, HistoryError> {
        let Some(digest) = value.strip_prefix("ath_public_v1_") else {
            return Err(HistoryError::Invalid("public ID"));
        };
        if !super::record::is_digest(digest) {
            return Err(HistoryError::Invalid("public ID"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct EstablishmentOrdinal {
    pub sequence: u64,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceObjectKey {
    pub namespace: SourceNamespace,
    pub identifier: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum EffectiveScope {
    Unknown,
    Closed(ProfileDateRangeV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BindingClaim {
    pub id: String,
    pub source: SourceObjectKey,
    pub candidate: AthleteCandidateId,
    pub effective: EffectiveScope,
    pub supersedes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BindingPartition {
    pub binding_ids: Vec<String>,
    pub representative: AthleteCandidateId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SplitRequest {
    pub canonical: PublicAthleteId,
    pub revoke_merges: Vec<u64>,
    pub partitions: Vec<BindingPartition>,
    pub unresolved_bindings: Vec<String>,
    pub affected_record_keys: Vec<String>,
    pub evidence: CandidateDecisionReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RevocationRequest {
    pub canonical: PublicAthleteId,
    pub revoke_merges: Vec<u64>,
    pub affected_record_keys: Vec<String>,
    pub evidence: CandidateDecisionReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum HistoryRequest {
    Establish {
        binding: BindingClaim,
        evidence: CandidateDecisionReference,
    },
    EstablishMatchedGroup {
        bindings: Vec<BindingClaim>,
        evidence: CandidateDecisionReference,
    },
    Bind {
        owner: PublicAthleteId,
        binding: BindingClaim,
        evidence: CandidateDecisionReference,
    },
    Merge {
        canonical: PublicAthleteId,
        members: Vec<PublicAthleteId>,
        evidence: CandidateDecisionReference,
    },
    Split(SplitRequest),
    Revoke(RevocationRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayLimits {
    pub records: usize,
    pub identities: usize,
    pub bindings: usize,
    pub members_per_operation: usize,
    pub text_bytes: usize,
    pub total_text_bytes: usize,
    pub traversal_steps: usize,
    pub graph_steps: usize,
    pub affected_record_keys: usize,
}

impl ReplayLimits {
    pub const SMALL: Self = Self {
        records: 4_096,
        identities: 4_096,
        bindings: 8_192,
        members_per_operation: 128,
        text_bytes: 512,
        total_text_bytes: 4 * 1024 * 1024,
        traversal_steps: 8_192,
        graph_steps: 1_000_000,
        affected_record_keys: 1_024,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSuccessor {
    pub public_id: PublicAthleteId,
    pub representative_candidate: AthleteCandidateId,
    pub established: EstablishmentOrdinal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicResolution {
    Unknown,
    Unresolved,
    Canonical(ResolvedSuccessor),
    Redirect {
        successor: ResolvedSuccessor,
        changed_at: u64,
    },
    RevokedRedirect {
        successor: ResolvedSuccessor,
        changed_at: u64,
        revoked_merges: Vec<u64>,
    },
    Ambiguous {
        successors: Vec<ResolvedSuccessor>,
        unresolved_bindings: bool,
        changed_at: u64,
        revoked_merges: Vec<u64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingResolution {
    Unknown,
    Unresolved,
    Resolved {
        identity: PublicResolution,
        established_at: u64,
        superseded_by: Option<String>,
    },
}
