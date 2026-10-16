use super::replay::MergeEntry;
use super::{
    CandidateDecisionReference, CandidateEvidenceKind, CandidateMembershipClaimV1, HistoryError,
    HistoryProjection, PublicAthleteId, PublicResolution,
};
use crate::model::AthleteCandidateId;

#[derive(Debug, Clone)]
pub(super) struct MembershipEntry {
    pub claim: CandidateMembershipClaimV1,
    pub owner: Option<PublicAthleteId>,
    pub established_at: u64,
}

impl HistoryProjection {
    pub fn membership_claim(&self, id: &str) -> Option<&CandidateMembershipClaimV1> {
        self.memberships.get(id).map(|entry| &entry.claim)
    }

    pub fn withheld_memberships(&self, sequence: u64) -> Option<&[String]> {
        self.withheld.get(&sequence).map(Vec::as_slice)
    }

    pub(super) fn claims(
        &self,
    ) -> impl Iterator<Item = (&str, &AthleteCandidateId, Option<&PublicAthleteId>)> {
        self.bindings
            .iter()
            .map(|(id, entry)| (id.as_str(), &entry.claim.candidate, entry.owner.as_ref()))
            .chain(
                self.memberships
                    .iter()
                    .map(|(id, entry)| (id.as_str(), &entry.claim.candidate, entry.owner.as_ref())),
            )
    }

    pub(super) fn claim_candidate(&self, id: &str) -> Option<&AthleteCandidateId> {
        self.bindings
            .get(id)
            .map(|entry| &entry.claim.candidate)
            .or_else(|| self.memberships.get(id).map(|entry| &entry.claim.candidate))
    }

    pub(super) fn assign_claim(
        &mut self,
        id: &str,
        owner: Option<PublicAthleteId>,
    ) -> Result<(), HistoryError> {
        if let Some(entry) = self.bindings.get_mut(id) {
            entry.owner = owner;
            return Ok(());
        }
        let entry = self
            .memberships
            .get_mut(id)
            .ok_or(HistoryError::Unknown("claim"))?;
        entry.owner = owner;
        Ok(())
    }

    pub(super) fn check_claim_capacity(&self) -> Result<(), HistoryError> {
        let count = self
            .bindings
            .len()
            .checked_add(self.memberships.len())
            .ok_or(HistoryError::CounterOverflow)?;
        if count >= self.limits.bindings {
            return Err(HistoryError::Limit("bindings"));
        }
        Ok(())
    }

    pub(super) fn component_has_membership(
        &self,
        owner: &PublicAthleteId,
    ) -> Result<bool, HistoryError> {
        self.memberships.values().try_fold(false, |found, entry| {
            let included = match &entry.owner {
                Some(held) => self.root(held)? == *owner,
                None => false,
            };
            Ok(found || included)
        })
    }

    pub(super) fn check_membership_merge_bound(
        &self,
        owners: &[PublicAthleteId],
    ) -> Result<(), HistoryError> {
        let includes_membership = self.memberships.values().try_fold(false, |found, entry| {
            let included = match &entry.owner {
                Some(owner) => owners.contains(&self.root(owner)?),
                None => false,
            };
            Ok::<_, HistoryError>(found || included)
        })?;
        if includes_membership {
            owners.iter().try_fold(0usize, |count, owner| {
                let total = count
                    .checked_add(self.component_bindings(owner)?.len())
                    .ok_or(HistoryError::CounterOverflow)?;
                if total > self.limits.members_per_operation {
                    return Err(HistoryError::Limit("affected bindings"));
                }
                Ok(total)
            })?;
        }
        Ok(())
    }

    pub(super) fn bind_candidate_membership(
        &mut self,
        owner: &PublicAthleteId,
        claim: &CandidateMembershipClaimV1,
        evidence: &CandidateDecisionReference,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        self.check_membership(owner, claim, evidence)?;
        self.memberships.insert(
            claim.id.clone(),
            MembershipEntry {
                claim: claim.clone(),
                owner: Some(owner.clone()),
                established_at: sequence,
            },
        );
        let entry = self
            .identities
            .get_mut(owner)
            .ok_or(HistoryError::Unknown("membership owner"))?;
        entry.owned_bindings.insert(claim.id.clone());
        entry.changed_at = sequence;
        self.merges.insert(
            sequence,
            MergeEntry {
                binding_ids: self
                    .claims()
                    .filter(|(_, candidate, _)| evidence.members.contains(candidate))
                    .map(|(id, _, _)| id.to_owned())
                    .collect(),
                revoked_at: None,
            },
        );
        Ok(())
    }

    fn check_membership(
        &self,
        owner: &PublicAthleteId,
        claim: &CandidateMembershipClaimV1,
        evidence: &CandidateDecisionReference,
    ) -> Result<(), HistoryError> {
        self.require_current(owner)?;
        self.check_claim_capacity()?;
        if self.bindings.contains_key(&claim.id) || self.memberships.contains_key(&claim.id) {
            return Err(HistoryError::Duplicate("binding ID"));
        }
        if self.resolve_candidate(&claim.candidate)? != PublicResolution::Unknown {
            return Err(HistoryError::BindingConflict);
        }
        let anchor = self
            .bindings
            .get(&claim.anchor_binding_id)
            .ok_or(HistoryError::Unknown("membership anchor"))?;
        if self
            .bindings
            .values()
            .filter(|binding| binding.claim.source == claim.source)
            .take(2)
            .count()
            != 1
        {
            return Err(HistoryError::SourceEpochEvidenceRequired);
        }
        if anchor.superseded_by.is_some()
            || anchor.claim.source != claim.source
            || self.candidate_root(&anchor.claim.candidate)? != *owner
        {
            return Err(HistoryError::BindingConflict);
        }
        if self.component_bindings(owner)?.len() >= self.limits.members_per_operation {
            return Err(HistoryError::Limit("affected bindings"));
        }
        if evidence.kind != CandidateEvidenceKind::SamePerson
            || evidence.members.len() < 2
            || !evidence.members.contains(&anchor.claim.candidate)
            || !evidence.members.contains(&claim.candidate)
        {
            return Err(HistoryError::EvidenceMismatch);
        }
        evidence
            .members
            .iter()
            .filter(|member| *member != &claim.candidate)
            .try_for_each(|member| {
                if self.candidate_root(member)? != *owner {
                    return Err(HistoryError::EvidenceMismatch);
                }
                Ok(())
            })
    }
}
