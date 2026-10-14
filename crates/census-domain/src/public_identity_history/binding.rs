use std::collections::BTreeSet;

use crate::model::{AthleteCandidateId, ProfileDate};

use super::replay::{BindingEntry, IdentityEntry, IdentityState, MergeEntry};
use super::{
    record, BindingClaim, BindingResolution, CandidateDecisionReference, CandidateEvidenceKind,
    EffectiveScope, HistoryError, HistoryProjection, PublicAthleteId, PublicResolution,
    SourceObjectKey,
};

impl HistoryProjection {
    pub fn resolve_candidate(
        &self,
        candidate: &AthleteCandidateId,
    ) -> Result<PublicResolution, HistoryError> {
        let Some(binding) = self
            .bindings
            .values()
            .find(|binding| &binding.claim.candidate == candidate)
        else {
            return Ok(PublicResolution::Unknown);
        };
        match &binding.owner {
            Some(owner) => self.resolve_public(owner),
            None => Ok(PublicResolution::Unresolved),
        }
    }

    pub fn binding_claim(&self, id: &str) -> Option<&BindingClaim> {
        self.bindings.get(id).map(|binding| &binding.claim)
    }

    pub fn resolve_binding(&self, id: &str) -> Result<BindingResolution, HistoryError> {
        let Some(binding) = self.bindings.get(id) else {
            return Ok(BindingResolution::Unknown);
        };
        let Some(owner) = &binding.owner else {
            return Ok(BindingResolution::Unresolved);
        };
        Ok(BindingResolution::Resolved {
            identity: self.resolve_public(owner)?,
            established_at: binding.established_at,
            superseded_by: binding.superseded_by.clone(),
        })
    }

    pub fn source_bindings_at(
        &self,
        source: &SourceObjectKey,
        date: &ProfileDate,
    ) -> Result<Vec<&str>, HistoryError> {
        self.bindings
            .values()
            .filter(|binding| &binding.claim.source == source)
            .try_fold(Vec::new(), |mut result, binding| {
                match &binding.claim.effective {
                    EffectiveScope::Unknown => return Err(HistoryError::BindingConflict),
                    EffectiveScope::Closed(range) if range.contains(date) => {
                        result
                            .try_reserve(1)
                            .map_err(|_| HistoryError::Allocation)?;
                        result.push(binding.claim.id.as_str());
                    }
                    EffectiveScope::Closed(_) => {}
                }
                Ok(result)
            })
    }

    pub(super) fn establish(
        &mut self,
        binding: &BindingClaim,
        evidence: &CandidateDecisionReference,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        if evidence.kind != CandidateEvidenceKind::SourceBound
            || !matches!(evidence.members.as_slice(), [member] if member == &binding.candidate)
        {
            return Err(HistoryError::EvidenceMismatch);
        }
        self.check_binding(binding)?;
        if self
            .bindings
            .values()
            .any(|held| held.claim.candidate == binding.candidate)
        {
            return Err(HistoryError::BindingConflict);
        }
        let id = record::public_id(&self.installation, sequence, 0)?;
        let entry = IdentityEntry {
            ordinal: record::ordinal(sequence, 0)?,
            representative: binding.candidate.clone(),
            owned_bindings: BTreeSet::from([binding.id.clone()]),
            state: IdentityState::Current,
            revoked: Vec::new(),
            changed_at: sequence,
        };
        if self.identities.insert(id.clone(), entry).is_some() {
            return Err(HistoryError::Duplicate("public ID"));
        }
        self.insert_binding(binding, id, sequence)
    }

    pub(super) fn bind(
        &mut self,
        owner: &PublicAthleteId,
        binding: &BindingClaim,
        evidence: &CandidateDecisionReference,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        self.require_current(owner)?;
        self.check_binding(binding)?;
        if evidence.kind != CandidateEvidenceKind::SamePerson
            || evidence.members.len() < 2
            || !evidence.members.contains(&binding.candidate)
        {
            return Err(HistoryError::EvidenceMismatch);
        }
        evidence
            .members
            .iter()
            .filter(|member| *member != &binding.candidate)
            .try_for_each(|member| {
                if self.candidate_root(member)? != *owner {
                    return Err(HistoryError::EvidenceMismatch);
                }
                Ok(())
            })?;
        if self
            .bindings
            .values()
            .any(|held| held.claim.candidate == binding.candidate)
        {
            return Err(HistoryError::BindingConflict);
        }
        self.identities
            .get_mut(owner)
            .ok_or(HistoryError::Unknown("binding owner"))?
            .owned_bindings
            .insert(binding.id.clone());
        self.identities
            .get_mut(owner)
            .ok_or(HistoryError::Unknown("binding owner"))?
            .changed_at = sequence;
        self.insert_binding(binding, owner.clone(), sequence)?;
        let binding_ids = self
            .bindings
            .values()
            .filter(|held| evidence.members.contains(&held.claim.candidate))
            .map(|held| held.claim.id.clone())
            .collect();
        self.merges.insert(
            sequence,
            MergeEntry {
                binding_ids,
                revoked_at: None,
            },
        );
        Ok(())
    }

    pub(super) fn check_binding(&self, binding: &BindingClaim) -> Result<(), HistoryError> {
        if self.bindings.len() >= self.limits.bindings {
            return Err(HistoryError::Limit("bindings"));
        }
        if self.bindings.contains_key(&binding.id) {
            return Err(HistoryError::Duplicate("binding ID"));
        }
        let prior: Vec<_> = self
            .bindings
            .values()
            .filter(|held| held.claim.source == binding.source)
            .collect();
        if prior.is_empty() {
            return if binding.supersedes.is_none() {
                Ok(())
            } else {
                Err(HistoryError::SourceReuse)
            };
        }
        let Some(supersedes) = &binding.supersedes else {
            return Err(HistoryError::SourceReuse);
        };
        let active: Vec<_> = prior
            .iter()
            .filter(|held| held.superseded_by.is_none())
            .collect();
        let [held] = active.as_slice() else {
            return Err(HistoryError::SourceReuse);
        };
        if held.claim.id != *supersedes
            || !strictly_later(&held.claim.effective, &binding.effective)
        {
            return Err(HistoryError::SourceReuse);
        }
        Ok(())
    }

    pub(super) fn insert_binding(
        &mut self,
        claim: &BindingClaim,
        owner: PublicAthleteId,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        if let Some(prior) = &claim.supersedes {
            let held = self
                .bindings
                .get_mut(prior)
                .ok_or(HistoryError::Unknown("superseded binding"))?;
            held.superseded_by = Some(claim.id.clone());
        }
        self.bindings.insert(
            claim.id.clone(),
            BindingEntry {
                claim: claim.clone(),
                owner: Some(owner),
                established_at: sequence,
                superseded_by: None,
            },
        );
        Ok(())
    }

    pub(super) fn candidate_root(
        &self,
        candidate: &AthleteCandidateId,
    ) -> Result<PublicAthleteId, HistoryError> {
        let roots = self
            .bindings
            .values()
            .filter(|binding| &binding.claim.candidate == candidate)
            .try_fold(BTreeSet::new(), |mut roots, binding| {
                let owner = binding
                    .owner
                    .as_ref()
                    .ok_or(HistoryError::BindingConflict)?;
                roots.insert(self.root(owner)?);
                Ok::<_, HistoryError>(roots)
            })?;
        if roots.len() != 1 {
            return Err(HistoryError::BindingConflict);
        }
        roots
            .into_iter()
            .next()
            .ok_or(HistoryError::BindingConflict)
    }

    pub(super) fn component_bindings(
        &self,
        canonical: &PublicAthleteId,
    ) -> Result<BTreeSet<String>, HistoryError> {
        self.bindings
            .iter()
            .try_fold(BTreeSet::new(), |mut found, (id, binding)| {
                if let Some(owner) = &binding.owner {
                    if self.root(owner)? == *canonical {
                        if found.len() >= self.limits.members_per_operation {
                            return Err(HistoryError::Limit("affected bindings"));
                        }
                        found.insert(id.clone());
                    }
                }
                Ok(found)
            })
    }
}

fn strictly_later(prior: &EffectiveScope, next: &EffectiveScope) -> bool {
    matches!((prior, next), (EffectiveScope::Closed(prior), EffectiveScope::Closed(next))
        if prior.through() < next.from())
}
