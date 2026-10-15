use super::*;

pub(super) struct Semantic<'a> {
    pub operation: &'a str,
    pub contexts: Option<(&'a str, &'a str)>,
    pub observed_at: &'a str,
    pub expected_head: &'a RegistryHead,
    pub members: &'a [AthleteCandidateId],
    pub request: &'a LocalMembershipWithholdV1,
    pub dispositions: &'a [RetainedClaimDispositionV1],
}

impl Semantic<'_> {
    pub fn validate(&self, claims: &[RetainedClaimV1]) -> Result<(), HistoryError> {
        if self.request.attachment.sequence == 0
            || self.request.attachment.sequence > self.expected_head.sequence
            || !super::super::record::is_digest(&self.request.attachment.digest)
            || self.members.len() < 2
            || self.members.len() > 128
            || !ordered(self.members)
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
            self.operation,
            self.request.case_id.as_str(),
            self.request.attachment.membership_id.as_str(),
        ] {
            text(value, 512)?;
        }
        if let Some((authority, affected)) = self.contexts {
            text(authority, 512)?;
            text(affected, 512)?;
        }
        text(&self.request.rationale, 4096)?;
        text(self.observed_at, 64)?;
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
        let rejected = claims.iter().zip(self.dispositions).try_fold(
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
pub(super) fn text(value: &str, limit: usize) -> Result<(), HistoryError> {
    if value.trim().is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        return Err(HistoryError::Invalid("membership withholding text"));
    }
    Ok(())
}
