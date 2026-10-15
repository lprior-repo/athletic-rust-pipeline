use super::*;
mod codec;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalMembershipWithholdingRecordV2 {
    pub version: u32,
    pub policy: String,
    pub store: StoreIdentity,
    pub operation: String,
    pub expected_head: RegistryHead,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub graph: crate::public_identity_history::GraphAuthorityReferenceV1,
    pub members: Vec<AthleteCandidateId>,
    pub request: LocalMembershipWithholdV1,
    pub observed_at: String,
    pub dispositions: Vec<RetainedClaimDispositionV1>,
}

impl LocalMembershipWithholdingRecordV2 {
    pub fn history_request(
        &self,
        claims: &[RetainedClaimV1],
    ) -> Result<HistoryRequest, HistoryError> {
        self.validate(claims)?;
        let digest = serialized_digest(&(LOCAL_MEMBERSHIP_WITHHOLDING_GRAPH_POLICY, self))
            .map_err(|error| HistoryError::Encoding(error.to_string()))?;
        Ok(HistoryRequest::Revoke(RevocationRequest {
            canonical: self.request.canonical.clone(),
            revoke_merges: vec![self.request.attachment.sequence],
            affected_record_keys: self.request.affected_record_keys.clone(),
            evidence: CandidateDecisionReference {
                application_id: format!("local_membership_withholding_v2_{digest}"),
                evidence_digest: digest,
                kind: CandidateEvidenceKind::Correction,
                members: self.members.clone(),
            },
        }))
    }

    pub fn validate(&self, claims: &[RetainedClaimV1]) -> Result<(), HistoryError> {
        if self.version != 2 || self.policy != LOCAL_MEMBERSHIP_WITHHOLDING_GRAPH_POLICY {
            return Err(HistoryError::Invalid("membership withholding inventory"));
        }
        self.graph.validate()?;
        super::semantic::Semantic {
            operation: &self.operation,
            contexts: None,
            observed_at: &self.observed_at,
            expected_head: &self.expected_head,
            members: &self.members,
            request: &self.request,
            dispositions: &self.dispositions,
        }
        .validate(claims)
    }
}
