use crate::model::{AppliedAthleteIdentity, SourceNamespace};

use super::{
    BindingClaim, CandidateDecisionReference, HistoryError, HistoryRequest, PublicAthleteId,
    ReplayLimits, RevocationRequest, SplitRequest,
};

pub(super) fn request(
    request: &HistoryRequest,
    limits: ReplayLimits,
) -> Result<usize, HistoryError> {
    let mut budget = Budget { limits, used: 0 };
    match request {
        HistoryRequest::Establish { binding, evidence } => {
            budget.admit_establish(binding, evidence)?
        }
        HistoryRequest::EstablishMatchedGroup { bindings, evidence } => {
            budget.admit_matched_group(bindings, evidence)?
        }
        HistoryRequest::Bind {
            owner,
            binding,
            evidence,
        } => budget.admit_bind(owner, binding, evidence)?,
        HistoryRequest::BindCandidateMembership {
            owner,
            membership,
            evidence,
        } => {
            budget.text(owner.as_str())?;
            budget.text(&membership.id)?;
            budget.text(membership.candidate.as_str())?;
            budget.text(&membership.anchor_binding_id)?;
            budget.text(&membership.source.identifier)?;
            budget.namespace(&membership.source.namespace)?;
            budget.evidence(evidence)?;
        }
        HistoryRequest::Merge {
            canonical,
            members,
            evidence,
        } => budget.admit_merge(canonical, members, evidence)?,
        HistoryRequest::Split(split) => budget.split(split)?,
        HistoryRequest::Revoke(revocation) => budget.admit_revocation(revocation)?,
    }
    Ok(budget.used)
}

struct Budget {
    limits: ReplayLimits,
    used: usize,
}

pub(super) fn accepted_reference(
    record: &AppliedAthleteIdentity,
    limits: ReplayLimits,
) -> Result<(), HistoryError> {
    let mut budget = Budget { limits, used: 0 };
    budget.count(
        record.members.len(),
        limits.members_per_operation,
        "evidence members",
    )?;
    budget.text(&record.id)?;
    record.members.iter().try_for_each(|member| {
        budget.text(member.subject.as_str())?;
        budget.text(&member.evidence_digest)
    })?;
    if let Some(canonical) = &record.canonical_id {
        budget.text(canonical.as_str())?;
    }
    if let Some(case_id) = &record.case_id {
        budget.text(case_id)?;
    }
    if let Some(verdict) = &record.verdict_digest {
        budget.text(verdict)?;
    }
    Ok(())
}

impl Budget {
    fn admit_establish(
        &mut self,
        binding: &BindingClaim,
        evidence: &CandidateDecisionReference,
    ) -> Result<(), HistoryError> {
        self.binding(binding)?;
        self.evidence(evidence)
    }

    fn admit_matched_group(
        &mut self,
        bindings: &[BindingClaim],
        evidence: &CandidateDecisionReference,
    ) -> Result<(), HistoryError> {
        self.count(
            bindings.len(),
            self.limits.members_per_operation,
            "matched group bindings",
        )?;
        self.count(bindings.len(), self.limits.bindings, "bindings")?;
        bindings
            .iter()
            .try_for_each(|binding| self.binding(binding))?;
        self.evidence(evidence)
    }

    fn admit_bind(
        &mut self,
        owner: &PublicAthleteId,
        binding: &BindingClaim,
        evidence: &CandidateDecisionReference,
    ) -> Result<(), HistoryError> {
        self.text(owner.as_str())?;
        self.binding(binding)?;
        self.evidence(evidence)
    }

    fn admit_merge(
        &mut self,
        canonical: &PublicAthleteId,
        members: &[PublicAthleteId],
        evidence: &CandidateDecisionReference,
    ) -> Result<(), HistoryError> {
        self.count(
            members.len(),
            self.limits.members_per_operation,
            "merge members",
        )?;
        self.text(canonical.as_str())?;
        members
            .iter()
            .try_for_each(|member| self.text(member.as_str()))?;
        self.evidence(evidence)
    }

    fn admit_revocation(&mut self, revocation: &RevocationRequest) -> Result<(), HistoryError> {
        self.count(
            revocation.revoke_merges.len(),
            self.limits.members_per_operation,
            "revocations",
        )?;
        self.count(
            revocation.affected_record_keys.len(),
            self.limits.affected_record_keys,
            "affected records",
        )?;
        self.text(revocation.canonical.as_str())?;
        revocation
            .affected_record_keys
            .iter()
            .try_for_each(|id| self.text(id))?;
        self.evidence(&revocation.evidence)
    }

    fn count(&self, count: usize, limit: usize, field: &'static str) -> Result<(), HistoryError> {
        if count > limit {
            return Err(HistoryError::Limit(field));
        }
        Ok(())
    }

    fn text(&mut self, text: &str) -> Result<(), HistoryError> {
        if text.is_empty() || text.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(HistoryError::Invalid("empty or control-containing text"));
        }
        self.count(text.len(), self.limits.text_bytes, "text bytes")?;
        self.used = self
            .used
            .checked_add(text.len())
            .ok_or(HistoryError::CounterOverflow)?;
        self.count(self.used, self.limits.total_text_bytes, "total text bytes")
    }

    fn evidence(&mut self, evidence: &CandidateDecisionReference) -> Result<(), HistoryError> {
        self.count(
            evidence.members.len(),
            self.limits.members_per_operation,
            "evidence members",
        )?;
        self.text(&evidence.application_id)?;
        self.text(&evidence.evidence_digest)?;
        if !super::record::is_digest(&evidence.evidence_digest) {
            return Err(HistoryError::Invalid("evidence digest"));
        }
        evidence
            .members
            .iter()
            .try_for_each(|member| self.text(member.as_str()))
    }

    fn binding(&mut self, binding: &BindingClaim) -> Result<(), HistoryError> {
        self.text(&binding.id)?;
        self.text(&binding.source.identifier)?;
        self.text(binding.candidate.as_str())?;
        if let Some(prior) = &binding.supersedes {
            self.text(prior)?;
        }
        self.namespace(&binding.source.namespace)
    }

    fn namespace(&mut self, namespace: &SourceNamespace) -> Result<(), HistoryError> {
        match namespace {
            SourceNamespace::MilesplitAthlete
            | SourceNamespace::TfrrsAthlete
            | SourceNamespace::DirectAthleticsAthlete => Ok(()),
            SourceNamespace::AssociationAthlete { association } => self.text(association),
            SourceNamespace::TimerAthlete { provider } => self.text(provider),
            SourceNamespace::AthleticNet { kind } if kind == "athlete" => Ok(()),
            _ => Err(HistoryError::Invalid("athlete source namespace")),
        }
    }

    fn split(&mut self, split: &SplitRequest) -> Result<(), HistoryError> {
        self.count(
            split.partitions.len(),
            self.limits.members_per_operation,
            "split partitions",
        )?;
        self.count(
            split.revoke_merges.len(),
            self.limits.members_per_operation,
            "revocations",
        )?;
        self.count(
            split.unresolved_bindings.len(),
            self.limits.members_per_operation,
            "unresolved bindings",
        )?;
        self.count(
            split.affected_record_keys.len(),
            self.limits.affected_record_keys,
            "affected records",
        )?;
        let bindings = split.partitions.iter().try_fold(
            split.unresolved_bindings.len(),
            |count, partition| {
                count
                    .checked_add(partition.binding_ids.len())
                    .ok_or(HistoryError::CounterOverflow)
            },
        )?;
        self.count(
            bindings,
            self.limits.members_per_operation,
            "affected bindings",
        )?;
        self.text(split.canonical.as_str())?;
        split.partitions.iter().try_for_each(|partition| {
            self.count(
                partition.binding_ids.len(),
                self.limits.members_per_operation,
                "partition bindings",
            )?;
            self.text(partition.representative.as_str())?;
            partition
                .binding_ids
                .iter()
                .try_for_each(|id| self.text(id))
        })?;
        split
            .unresolved_bindings
            .iter()
            .try_for_each(|id| self.text(id))?;
        split
            .affected_record_keys
            .iter()
            .try_for_each(|id| self.text(id))?;
        self.evidence(&split.evidence)
    }
}
