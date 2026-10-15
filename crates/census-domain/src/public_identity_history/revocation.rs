use std::collections::BTreeSet;

use super::{
    CandidateDecisionReference, CandidateEvidenceKind, HistoryError, HistoryProjection,
    RevocationRequest,
};

impl HistoryProjection {
    pub(super) fn revoke(
        &mut self,
        request: &RevocationRequest,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        self.require_current(&request.canonical)?;
        let affected = self.component_bindings(&request.canonical)?;
        self.check_correction(
            &request.evidence,
            &affected,
            &request.revoke_merges,
            &request.affected_record_keys,
        )?;
        let mut work = 0;
        self.check_revoked_edges(&request.revoke_merges, &affected, &mut work)?;
        let withheld = self.membership_detachment(&affected, &request.revoke_merges, &mut work)?;
        let predecessors = self.split_predecessors(&request.canonical)?;
        predecessors.iter().try_for_each(|id| {
            let entry = self
                .identities
                .get_mut(id)
                .ok_or(HistoryError::Unknown("revocation predecessor"))?;
            entry.revoked =
                super::split::retained_revocations(&entry.revoked, &request.revoke_merges);
            entry.changed_at = sequence;
            Ok::<_, HistoryError>(())
        })?;
        request.revoke_merges.iter().try_for_each(|merge| {
            self.merges
                .get_mut(merge)
                .ok_or(HistoryError::Unknown("revoked merge"))?
                .revoked_at = Some(sequence);
            Ok::<_, HistoryError>(())
        })?;
        withheld
            .iter()
            .try_for_each(|id| self.assign_claim(id, None))?;
        self.withheld.insert(sequence, withheld);
        self.affected
            .insert(sequence, request.affected_record_keys.clone());
        Ok(())
    }

    pub(super) fn check_correction(
        &self,
        evidence: &CandidateDecisionReference,
        affected: &BTreeSet<String>,
        revoked: &[u64],
        keys: &[String],
    ) -> Result<(), HistoryError> {
        if evidence.kind != CandidateEvidenceKind::Correction
            || revoked.is_empty()
            || keys.is_empty()
        {
            return Err(HistoryError::EvidenceMismatch);
        }
        if affected.len() > self.limits.members_per_operation {
            return Err(HistoryError::Limit("affected bindings"));
        }
        let members: BTreeSet<_> = affected
            .iter()
            .map(|id| {
                self.claim_candidate(id)
                    .cloned()
                    .ok_or(HistoryError::Unknown("affected binding"))
            })
            .collect::<Result<_, _>>()?;
        if members != evidence.members.iter().cloned().collect() {
            return Err(HistoryError::EvidenceMismatch);
        }
        Ok(())
    }

    pub(super) fn check_revoked_edges(
        &self,
        revoked: &[u64],
        affected: &BTreeSet<String>,
        work: &mut usize,
    ) -> Result<(), HistoryError> {
        revoked.iter().try_for_each(|sequence| {
            let merge = self
                .merges
                .get(sequence)
                .ok_or(HistoryError::Unknown("merge event"))?;
            let steps = merge
                .binding_ids
                .len()
                .checked_add(affected.len())
                .and_then(|steps| steps.checked_add(1))
                .ok_or(HistoryError::CounterOverflow)?;
            self.charge_graph_work(steps, work)?;
            if merge.revoked_at.is_some() || !merge.binding_ids.is_subset(affected) {
                return Err(HistoryError::Revocation);
            }
            Ok(())
        })
    }
}
