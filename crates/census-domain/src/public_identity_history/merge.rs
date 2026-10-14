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
        if members.is_empty()
            || evidence.members.len() < 2
            || evidence.kind != CandidateEvidenceKind::SamePerson
        {
            return Err(HistoryError::EvidenceMismatch);
        }
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
        let evidence_roots = evidence
            .members
            .iter()
            .map(|candidate| self.candidate_root(candidate))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if evidence_roots != members.iter().cloned().collect() {
            return Err(HistoryError::EvidenceMismatch);
        }
        let bindings = self
            .bindings
            .values()
            .filter(|binding| evidence.members.contains(&binding.claim.candidate))
            .map(|binding| binding.claim.id.clone())
            .collect();
        let revoked: BTreeSet<_> = members
            .iter()
            .filter_map(|member| self.identities.get(member))
            .flat_map(|entry| entry.revoked.iter().copied())
            .collect();
        self.identities
            .get_mut(canonical)
            .ok_or(HistoryError::Unknown("merge target"))?
            .revoked = revoked.into_iter().collect();
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
                binding_ids: bindings,
                revoked_at: None,
            },
        );
        Ok(())
    }
}
