use crate::model::{serialized_digest, AthleteCandidateId, StoreIdentity};
use serde::{Deserialize, Serialize};

use super::{
    BindingPartition, CandidateDecisionReference, CandidateEvidenceKind, HistoryError,
    HistoryRequest, PublicAthleteId, RegistryHead, SplitRequest,
};

pub(super) mod codec;
mod validate;

pub const LOCAL_ADJUDICATION_POLICY: &str = "local-operator-retained-identity-review/v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedClaimV1 {
    pub candidate: AthleteCandidateId,
    pub section: RetainedIdentitySectionV1,
    pub ordinal: u64,
    pub row_digest: String,
    pub claim_path: String,
    pub claim_digest: String,
    pub retained_value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetainedIdentitySectionV1 {
    Athlete,
    SourceObservation,
    CandidateMembership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimTreatmentV1 {
    RetainAsContext,
    RejectIdentityCorroboration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedClaimDispositionV1 {
    pub claim: RetainedClaimV1,
    pub treatment: ClaimTreatmentV1,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupersededMergeV1 {
    pub sequence: u64,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalIdentitySplitV1 {
    pub case_id: String,
    pub canonical: PublicAthleteId,
    pub supersedes: Vec<SupersededMergeV1>,
    pub partitions: Vec<BindingPartition>,
    pub unresolved_bindings: Vec<String>,
    pub affected_record_keys: Vec<String>,
    pub dispositions: Vec<RetainedClaimDispositionV1>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalIdentityAdjudicationRecordV2 {
    pub version: u32,
    pub policy: String,
    pub store: StoreIdentity,
    pub operation: String,
    pub expected_head: RegistryHead,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub authority_context: String,
    pub affected_context: String,
    pub members: Vec<AthleteCandidateId>,
    pub request: LocalIdentitySplitV1,
    pub observed_at: String,
    pub evidence_basis: LocalEvidenceBasisV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocalEvidenceBasisV1 {
    RetainedObservationOnly,
}

impl LocalIdentityAdjudicationRecordV2 {
    pub fn history_request(
        &self,
        claims: &[RetainedClaimV1],
    ) -> Result<HistoryRequest, HistoryError> {
        self.validate(claims)?;
        let digest = serialized_digest(&(LOCAL_ADJUDICATION_POLICY, self))
            .map_err(|error| HistoryError::Encoding(error.to_string()))?;
        Ok(HistoryRequest::Split(SplitRequest {
            canonical: self.request.canonical.clone(),
            revoke_merges: self
                .request
                .supersedes
                .iter()
                .map(|row| row.sequence)
                .collect(),
            partitions: self.request.partitions.clone(),
            unresolved_bindings: self.request.unresolved_bindings.clone(),
            affected_record_keys: self.request.affected_record_keys.clone(),
            evidence: CandidateDecisionReference {
                application_id: format!("local_adjudication_v1_{digest}"),
                evidence_digest: digest,
                kind: CandidateEvidenceKind::Correction,
                members: self.members.clone(),
            },
        }))
    }
}
