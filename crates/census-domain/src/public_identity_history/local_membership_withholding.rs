use super::local_adjudication::{
    ClaimTreatmentV1, RetainedClaimDispositionV1, RetainedClaimV1, RetainedIdentitySectionV1,
};
use super::{
    CandidateDecisionReference, CandidateEvidenceKind, HistoryError, HistoryRequest,
    PublicAthleteId, RegistryHead, RevocationRequest,
};
use crate::model::{serialized_digest, AthleteCandidateId, StoreIdentity};
use serde::Serialize;

mod codec;
mod graph_record;
mod semantic;
pub use graph_record::LocalMembershipWithholdingRecordV2;
pub const LOCAL_MEMBERSHIP_WITHHOLDING_GRAPH_POLICY: &str =
    "local-operator-source-membership-withholding/v2";
pub const LOCAL_MEMBERSHIP_WITHHOLDING_POLICY: &str =
    "local-operator-source-membership-withholding/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipAttachmentV1 {
    pub sequence: u64,
    pub digest: String,
    pub membership_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalMembershipWithholdV1 {
    pub case_id: String,
    pub canonical: PublicAthleteId,
    pub attachment: MembershipAttachmentV1,
    pub expected_withheld_memberships: Vec<String>,
    pub affected_record_keys: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalMembershipWithholdingRecordV1 {
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
    pub request: LocalMembershipWithholdV1,
    pub observed_at: String,
    pub dispositions: Vec<RetainedClaimDispositionV1>,
}

impl LocalMembershipWithholdingRecordV1 {
    pub fn history_request(
        &self,
        claims: &[RetainedClaimV1],
    ) -> Result<HistoryRequest, HistoryError> {
        self.validate(claims)?;
        let digest = serialized_digest(&(LOCAL_MEMBERSHIP_WITHHOLDING_POLICY, self))
            .map_err(|error| HistoryError::Encoding(error.to_string()))?;
        Ok(HistoryRequest::Revoke(RevocationRequest {
            canonical: self.request.canonical.clone(),
            revoke_merges: vec![self.request.attachment.sequence],
            affected_record_keys: self.request.affected_record_keys.clone(),
            evidence: CandidateDecisionReference {
                application_id: format!("local_membership_withholding_v1_{digest}"),
                evidence_digest: digest,
                kind: CandidateEvidenceKind::Correction,
                members: self.members.clone(),
            },
        }))
    }

    pub fn validate(&self, claims: &[RetainedClaimV1]) -> Result<(), HistoryError> {
        if self.version != 1 || self.policy != LOCAL_MEMBERSHIP_WITHHOLDING_POLICY {
            return Err(HistoryError::Invalid("membership withholding inventory"));
        }
        semantic::Semantic {
            operation: &self.operation,
            contexts: Some((&self.authority_context, &self.affected_context)),
            observed_at: &self.observed_at,
            expected_head: &self.expected_head,
            members: &self.members,
            request: &self.request,
            dispositions: &self.dispositions,
        }
        .validate(claims)
    }
}
