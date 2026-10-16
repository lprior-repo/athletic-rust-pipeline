use crate::public_identity::{codec, PublicIdentityError};
use crate::StoreResult;
use census_domain::model::{
    serialized_digest, AcceptedAthleteIdentity, AppliedIdentityKind, StoreIdentity,
};
use census_domain::public_identity_history::{
    BindingPartition, CandidateDecisionReference, HistoryRequest, PublicAthleteId, ReplayLimits,
    RevocationRequest, SplitRequest,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ReviewedIdentityRequestV1 {
    Merge {
        case_id: String,
        canonical: PublicAthleteId,
        members: Vec<PublicAthleteId>,
    },
    Split {
        case_id: String,
        canonical: PublicAthleteId,
        revoke_merges: Vec<u64>,
        partitions: Vec<BindingPartition>,
        unresolved_bindings: Vec<String>,
        affected_record_keys: Vec<String>,
    },
    Revoke {
        case_id: String,
        canonical: PublicAthleteId,
        revoke_merges: Vec<u64>,
        affected_record_keys: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::public_identity) enum ReviewedKind {
    Merge,
    Split,
    Revoke,
}

impl ReviewedIdentityRequestV1 {
    pub(in crate::public_identity) fn case_id(&self) -> &str {
        match self {
            Self::Merge { case_id, .. }
            | Self::Split { case_id, .. }
            | Self::Revoke { case_id, .. } => case_id,
        }
    }
    pub(in crate::public_identity) fn kind(&self) -> ReviewedKind {
        match self {
            Self::Merge { .. } => ReviewedKind::Merge,
            Self::Split { .. } => ReviewedKind::Split,
            Self::Revoke { .. } => ReviewedKind::Revoke,
        }
    }
    pub(in crate::public_identity) fn authorized(
        &self,
        accepted: &AcceptedAthleteIdentity,
    ) -> StoreResult<HistoryRequest> {
        if accepted.record().case_id.as_deref() != Some(self.case_id()) {
            return Err(PublicIdentityError::NotReviewed.into());
        }
        let evidence = match (self.kind(), accepted.record().kind) {
            (ReviewedKind::Merge, AppliedIdentityKind::SamePerson) => {
                CandidateDecisionReference::from_accepted(accepted, ReplayLimits::SMALL)
            }
            (ReviewedKind::Split | ReviewedKind::Revoke, AppliedIdentityKind::DifferentPerson) => {
                CandidateDecisionReference::from_correction(accepted, ReplayLimits::SMALL)
            }
            _ => return Err(PublicIdentityError::NotReviewed.into()),
        }
        .map_err(PublicIdentityError::from)?;
        Ok(match self {
            Self::Merge {
                canonical, members, ..
            } => HistoryRequest::Merge {
                canonical: canonical.clone(),
                members: members.clone(),
                evidence,
            },
            Self::Split {
                canonical,
                revoke_merges,
                partitions,
                unresolved_bindings,
                affected_record_keys,
                ..
            } => HistoryRequest::Split(SplitRequest {
                canonical: canonical.clone(),
                revoke_merges: revoke_merges.clone(),
                partitions: partitions.clone(),
                unresolved_bindings: unresolved_bindings.clone(),
                affected_record_keys: affected_record_keys.clone(),
                evidence,
            }),
            Self::Revoke {
                canonical,
                revoke_merges,
                affected_record_keys,
                ..
            } => HistoryRequest::Revoke(RevocationRequest {
                canonical: canonical.clone(),
                revoke_merges: revoke_merges.clone(),
                affected_record_keys: affected_record_keys.clone(),
                evidence,
            }),
        })
    }
    pub(in crate::public_identity) fn from_history(
        case_id: &str,
        request: &HistoryRequest,
    ) -> StoreResult<Self> {
        Ok(match request {
            HistoryRequest::Merge {
                canonical, members, ..
            } => Self::Merge {
                case_id: case_id.into(),
                canonical: canonical.clone(),
                members: members.clone(),
            },
            HistoryRequest::Split(split) => Self::Split {
                case_id: case_id.into(),
                canonical: split.canonical.clone(),
                revoke_merges: split.revoke_merges.clone(),
                partitions: split.partitions.clone(),
                unresolved_bindings: split.unresolved_bindings.clone(),
                affected_record_keys: split.affected_record_keys.clone(),
            },
            HistoryRequest::Revoke(revocation) => Self::Revoke {
                case_id: case_id.into(),
                canonical: revocation.canonical.clone(),
                revoke_merges: revocation.revoke_merges.clone(),
                affected_record_keys: revocation.affected_record_keys.clone(),
            },
            _ => return Err(PublicIdentityError::Corrupt("unsupported reviewed operation").into()),
        })
    }
}

pub(in crate::public_identity) fn digest(
    identity: &StoreIdentity,
    operation: &str,
    request: &ReviewedIdentityRequestV1,
) -> StoreResult<String> {
    if operation.is_empty()
        || operation.len() > 256
        || operation.chars().any(char::is_control)
        || request.case_id().is_empty()
        || request.case_id().len() > ReplayLimits::SMALL.text_bytes
        || request.case_id().chars().any(char::is_control)
    {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    let bounded = codec::encode(request, 65_536)?;
    Ok(serialized_digest(&(
        "public-identity-reviewed-request-v1",
        identity,
        operation,
        bounded,
    ))?)
}
