use super::*;
mod codec;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalIdentityAdjudicationRecordV3 {
    pub version: u32,
    pub policy: String,
    pub store: StoreIdentity,
    pub operation: String,
    pub expected_head: RegistryHead,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub graph: super::super::GraphAuthorityReferenceV1,
    pub members: Vec<AthleteCandidateId>,
    pub request: LocalIdentitySplitV1,
    pub observed_at: String,
    pub evidence_basis: LocalEvidenceBasisV1,
}

impl LocalIdentityAdjudicationRecordV3 {
    pub fn history_request(
        &self,
        claims: &[RetainedClaimV1],
    ) -> Result<HistoryRequest, HistoryError> {
        self.validate(claims)?;
        let digest = serialized_digest(&(LOCAL_ADJUDICATION_GRAPH_POLICY, self))
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
                application_id: format!("local_adjudication_v3_{digest}"),
                evidence_digest: digest,
                kind: CandidateEvidenceKind::Correction,
                members: self.members.clone(),
            },
        }))
    }
}

impl LocalIdentityAdjudicationRecordV3 {
    pub fn validate(&self, claims: &[RetainedClaimV1]) -> Result<(), HistoryError> {
        if self.version != 3 || self.policy != LOCAL_ADJUDICATION_GRAPH_POLICY {
            return Err(HistoryError::Invalid("manual adjudication graph policy"));
        }
        self.graph.validate()?;
        super::validate::semantic(
            &self.operation,
            &self.observed_at,
            &self.members,
            &self.request,
            claims,
        )
    }
}
