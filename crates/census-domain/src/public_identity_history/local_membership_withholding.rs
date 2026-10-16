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
        if self.version != 1
            || self.policy != LOCAL_MEMBERSHIP_WITHHOLDING_POLICY
            || self.request.attachment.sequence == 0
            || self.request.attachment.sequence > self.expected_head.sequence
            || !super::record::is_digest(&self.request.attachment.digest)
            || self.members.len() < 2
            || self.members.len() > 128
            || !ordered(&self.members)
            || self.request.expected_withheld_memberships.len() > 128
            || !ordered(&self.request.expected_withheld_memberships)
            || self.request.affected_record_keys.is_empty()
            || self.request.affected_record_keys.len() > 1024
            || !ordered(&self.request.affected_record_keys)
            || claims.is_empty()
            || claims.len() > 4096
            || claims.len() != self.dispositions.len()
        {
            return Err(HistoryError::Invalid("membership withholding inventory"));
        }
        for value in [
            &self.operation,
            &self.request.case_id,
            &self.request.attachment.membership_id,
            &self.authority_context,
            &self.affected_context,
        ] {
            text(value, 512)?;
        }
        text(&self.request.rationale, 4096)?;
        text(&self.observed_at, 64)?;
        self.request
            .affected_record_keys
            .iter()
            .try_for_each(|key| text(key, 1024))?;
        self.request
            .expected_withheld_memberships
            .iter()
            .try_for_each(|key| text(key, 512))?;
        self.validate_dispositions(claims)
    }

    fn validate_dispositions(&self, claims: &[RetainedClaimV1]) -> Result<(), HistoryError> {
        let rejected = claims.iter().zip(&self.dispositions).try_fold(
            0usize,
            |count, (claim, disposition)| {
                if claim != &disposition.claim || !self.members.contains(&claim.candidate) {
                    return Err(HistoryError::EvidenceMismatch);
                }
                text(&disposition.rationale, 4096)?;
                let target = claim.section == RetainedIdentitySectionV1::CandidateMembership
                    && claim.ordinal == self.request.attachment.sequence
                    && claim.row_digest == self.request.attachment.digest
                    && claim.claim_path == self.request.attachment.membership_id;
                let expected = if target {
                    ClaimTreatmentV1::RejectIdentityCorroboration
                } else {
                    ClaimTreatmentV1::RetainAsContext
                };
                if disposition.treatment != expected {
                    return Err(HistoryError::EvidenceMismatch);
                }
                count
                    .checked_add(usize::from(target))
                    .ok_or(HistoryError::CounterOverflow)
            },
        )?;
        if rejected != 1 {
            return Err(HistoryError::EvidenceMismatch);
        }
        Ok(())
    }
}

fn ordered<T: Ord>(rows: &[T]) -> bool {
    rows.windows(2).all(|pair| pair.first() < pair.get(1))
}
fn text(value: &str, limit: usize) -> Result<(), HistoryError> {
    if value.trim().is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        return Err(HistoryError::Invalid("membership withholding text"));
    }
    Ok(())
}
