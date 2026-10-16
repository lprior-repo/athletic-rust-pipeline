use super::AcceptedPublicIdentitySnapshot;
use crate::{PublicIdentityError, StoreResult};
use census_domain::public_identity_history::{
    BindingClaim, CandidateDecisionReference, HistoryRequest,
};

impl AcceptedPublicIdentitySnapshot {
    pub(crate) fn require_same_projection(&self, other: &Self) -> StoreResult<()> {
        if self.store_identity() != other.store_identity()
            || self.qualification_version() != other.qualification_version()
            || self.snapshot_sequence() != other.snapshot_sequence()
            || self.head() != other.head()
            || self.history_records() != other.history_records()
            || !self.public_ids().eq(other.public_ids())
            || !self
                .history
                .projection()
                .revoked_merges()
                .eq(other.history.projection().revoked_merges())
        {
            return Err(mismatch());
        }
        self.public_ids().try_for_each(|id| {
            if self.resolve_public(id)? != other.resolve_public(id)?
                || self.correction_history(id) != other.correction_history(id)
            {
                return Err(mismatch());
            }
            Ok(())
        })?;
        self.history_records().iter().try_for_each(|record| {
            if self.affected_record_keys(record.sequence)
                != other.affected_record_keys(record.sequence)
                || self
                    .history
                    .projection()
                    .withheld_memberships(record.sequence)
                    != other
                        .history
                        .projection()
                        .withheld_memberships(record.sequence)
            {
                return Err(mismatch());
            }
            self.compare_request(other, &record.request)
        })
    }

    fn compare_request(&self, other: &Self, request: &HistoryRequest) -> StoreResult<()> {
        match request {
            HistoryRequest::Establish { binding, evidence }
            | HistoryRequest::Bind {
                binding, evidence, ..
            } => {
                self.compare_binding(other, binding)?;
                self.compare_candidates(other, evidence)
            }
            HistoryRequest::EstablishMatchedGroup { bindings, evidence } => {
                bindings
                    .iter()
                    .try_for_each(|binding| self.compare_binding(other, binding))?;
                self.compare_candidates(other, evidence)
            }
            HistoryRequest::BindCandidateMembership {
                membership,
                evidence,
                ..
            } => {
                if self.history.projection().membership_claim(&membership.id)
                    != other.history.projection().membership_claim(&membership.id)
                {
                    return Err(mismatch());
                }
                self.compare_candidates(other, evidence)
            }
            HistoryRequest::Merge { evidence, .. } => self.compare_candidates(other, evidence),
            HistoryRequest::Split(split) => self.compare_candidates(other, &split.evidence),
            HistoryRequest::Revoke(revocation) => {
                self.compare_candidates(other, &revocation.evidence)
            }
        }
    }
    fn compare_binding(&self, other: &Self, binding: &BindingClaim) -> StoreResult<()> {
        if self.resolve_binding(&binding.id)? != other.resolve_binding(&binding.id)?
            || self.resolve_candidate(&binding.candidate)?
                != other.resolve_candidate(&binding.candidate)?
        {
            return Err(mismatch());
        }
        Ok(())
    }
    fn compare_candidates(
        &self,
        other: &Self,
        evidence: &CandidateDecisionReference,
    ) -> StoreResult<()> {
        evidence.members.iter().try_for_each(|candidate| {
            if self.resolve_candidate(candidate)? != other.resolve_candidate(candidate)? {
                return Err(mismatch());
            }
            Ok(())
        })
    }
}
fn mismatch() -> crate::StoreError {
    PublicIdentityError::Corrupt("frozen accepted identity projection differs").into()
}
