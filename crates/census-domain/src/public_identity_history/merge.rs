use std::collections::BTreeSet;

use super::replay::{IdentityState, MergeEntry};
use super::{
    CandidateDecisionReference, CandidateEvidenceKind, HistoryError, HistoryProjection,
    PublicAthleteId,
};

impl HistoryProjection {
    pub(super) fn merge(
        &mut self,
        canonical: &PublicAthleteId,
        members: &[PublicAthleteId],
        evidence: &CandidateDecisionReference,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        check_shape(members, evidence)?;
        self.require_oldest_is_canonical(canonical, members)?;
        self.check_coverage(members, evidence)?;
        self.check_membership_merge_bound(members)?;
        let binding_ids = self.merge_binding_ids(evidence);
        let carried = self.carried_revocations(members);
        self.apply_redirect(canonical, members, binding_ids, carried, sequence)
    }

    fn require_oldest_is_canonical(
        &self,
        canonical: &PublicAthleteId,
        members: &[PublicAthleteId],
    ) -> Result<(), HistoryError> {
        let oldest = members
            .iter()
            .map(|member| {
                self.require_current(member)
                    .map(|entry| (entry.ordinal, member))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .min();
        if !matches!(oldest, Some((_, oldest)) if oldest == canonical) {
            return Err(HistoryError::NonCanonicalTarget);
        }
        Ok(())
    }

    fn check_coverage(
        &self,
        members: &[PublicAthleteId],
        evidence: &CandidateDecisionReference,
    ) -> Result<(), HistoryError> {
        let evidence_roots = evidence
            .members
            .iter()
            .map(|candidate| self.candidate_root(candidate))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if evidence_roots != members.iter().cloned().collect() {
            return Err(HistoryError::EvidenceMismatch);
        }
        Ok(())
    }

    fn merge_binding_ids(&self, evidence: &CandidateDecisionReference) -> BTreeSet<String> {
        self.claims()
            .filter(|(_, candidate, _)| evidence.members.contains(candidate))
            .map(|(id, _, _)| id.to_owned())
            .collect()
    }

    fn carried_revocations(&self, members: &[PublicAthleteId]) -> Vec<u64> {
        let revoked: BTreeSet<u64> = members
            .iter()
            .filter_map(|member| self.identities.get(member))
            .flat_map(|entry| entry.revoked.iter().copied())
            .collect();
        revoked.into_iter().collect()
    }

    fn apply_redirect(
        &mut self,
        canonical: &PublicAthleteId,
        members: &[PublicAthleteId],
        binding_ids: BTreeSet<String>,
        carried: Vec<u64>,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        self.identities
            .get_mut(canonical)
            .ok_or(HistoryError::Unknown("merge target"))?
            .revoked = carried;
        self.identities
            .get_mut(canonical)
            .ok_or(HistoryError::Unknown("merge target"))?
            .changed_at = sequence;
        members
            .iter()
            .filter(|member| *member != canonical)
            .try_for_each(|member| {
                let entry = self
                    .identities
                    .get_mut(member)
                    .ok_or(HistoryError::Unknown("merge member"))?;
                entry.state = IdentityState::Redirect {
                    target: canonical.clone(),
                };
                entry.changed_at = sequence;
                Ok::<_, HistoryError>(())
            })?;
        self.merges.insert(
            sequence,
            MergeEntry {
                binding_ids,
                revoked_at: None,
            },
        );
        Ok(())
    }
}

fn check_shape(
    members: &[PublicAthleteId],
    evidence: &CandidateDecisionReference,
) -> Result<(), HistoryError> {
    if members.is_empty()
        || evidence.members.len() < 2
        || evidence.kind != CandidateEvidenceKind::SamePerson
    {
        return Err(HistoryError::EvidenceMismatch);
    }
    Ok(())
}
