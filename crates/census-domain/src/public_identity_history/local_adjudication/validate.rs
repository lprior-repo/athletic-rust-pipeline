use super::*;
use std::collections::BTreeSet;

const MAX_CLAIMS: usize = 4096;
const MAX_RATIONALE: usize = 4096;

impl LocalIdentityAdjudicationRecordV2 {
    pub fn validate(&self, claims: &[RetainedClaimV1]) -> Result<(), HistoryError> {
        if self.version != 2 || self.policy != LOCAL_ADJUDICATION_POLICY {
            return Err(HistoryError::Invalid("manual adjudication policy"));
        }
        semantic(
            &self.operation,
            &self.observed_at,
            &self.members,
            &self.request,
            claims,
        )
    }
}

pub(super) fn semantic(
    operation: &str,
    observed_at: &str,
    members: &[AthleteCandidateId],
    request: &LocalIdentitySplitV1,
    claims: &[RetainedClaimV1],
) -> Result<(), HistoryError> {
    text(operation, 256)?;
    text(observed_at, 64)?;
    text(&request.case_id, 512)?;
    text(&request.rationale, MAX_RATIONALE)?;
    if members.len() < 2
        || members.len() > super::super::ReplayLimits::SMALL.members_per_operation
        || !members.windows(2).all(|pair| pair.first() < pair.get(1))
        || claims.is_empty()
        || claims.len() > MAX_CLAIMS
        || request.dispositions.len() != claims.len()
        || request.supersedes.is_empty()
        || request.supersedes.len() > 128
        || request.partitions.len() > 128
        || request.unresolved_bindings.len() > 128
        || request.affected_record_keys.len() > 1024
    {
        return Err(HistoryError::Invalid("manual adjudication inventory"));
    }
    let mut cited = BTreeSet::new();
    let mut rejected = false;
    for (expected, disposition) in claims.iter().zip(&request.dispositions) {
        if expected != &disposition.claim || !members.contains(&expected.candidate) {
            return Err(HistoryError::EvidenceMismatch);
        }
        let key = serialized_digest(expected)
            .map_err(|error| HistoryError::Encoding(error.to_string()))?;
        if !cited.insert(key) {
            return Err(HistoryError::Invalid("duplicate manual claim"));
        }
        text(&disposition.rationale, MAX_RATIONALE)?;
        if disposition.treatment == ClaimTreatmentV1::RejectIdentityCorroboration {
            if !rejectable(expected) {
                return Err(HistoryError::Invalid("manual identity claim disposition"));
            }
            rejected = true;
        }
    }
    if !rejected {
        return Err(HistoryError::Invalid(
            "manual correction retains all corroboration",
        ));
    }
    Ok(())
}

fn rejectable(claim: &RetainedClaimV1) -> bool {
    claim.section == RetainedIdentitySectionV1::CandidateMembership
        || (claim.section == RetainedIdentitySectionV1::Athlete
            && (claim.claim_path.starts_with("/source_links/")
                || claim.claim_path.starts_with("/identity_attestations/")))
}

fn text(value: &str, maximum: usize) -> Result<(), HistoryError> {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return Err(HistoryError::Invalid("manual adjudication text"));
    }
    Ok(())
}
