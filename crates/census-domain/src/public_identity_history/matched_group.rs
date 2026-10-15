use std::collections::BTreeSet;

use crate::model::AthleteCandidateId;

use super::replay::{IdentityEntry, IdentityState, MergeEntry};
use super::{
    record, BindingClaim, CandidateDecisionReference, CandidateEvidenceKind, HistoryError,
    HistoryProjection,
};

impl HistoryProjection {
    pub(super) fn establish_matched_group(
        &mut self,
        bindings: &[BindingClaim],
        evidence: &CandidateDecisionReference,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        let representative = self.check_matched_group(bindings, evidence)?;
        let id = record::public_id(&self.installation, sequence, 0)?;
        let binding_ids: BTreeSet<_> = bindings.iter().map(|binding| binding.id.clone()).collect();
        let entry = IdentityEntry {
            ordinal: record::ordinal(sequence, 0)?,
            representative: representative.clone(),
            owned_bindings: binding_ids.clone(),
            state: IdentityState::Current,
            revoked: Vec::new(),
            changed_at: sequence,
        };
        if self.identities.insert(id.clone(), entry).is_some() {
            return Err(HistoryError::Duplicate("public ID"));
        }
        bindings
            .iter()
            .try_for_each(|binding| self.insert_binding(binding, id.clone(), sequence))?;
        self.merges.insert(
            sequence,
            MergeEntry {
                binding_ids,
                revoked_at: None,
            },
        );
        Ok(())
    }

    fn check_matched_group<'a>(
        &self,
        bindings: &[BindingClaim],
        evidence: &'a CandidateDecisionReference,
    ) -> Result<&'a AthleteCandidateId, HistoryError> {
        if evidence.kind != CandidateEvidenceKind::SamePerson
            || evidence.members.len() < 2
            || !bindings
                .iter()
                .map(|binding| &binding.candidate)
                .eq(evidence.members.iter())
        {
            return Err(HistoryError::EvidenceMismatch);
        }
        check_distinct_bindings(bindings)?;
        bindings.iter().try_for_each(|binding| {
            self.check_binding(binding)?;
            if self
                .claims()
                .any(|(_, candidate, _)| candidate == &binding.candidate)
            {
                return Err(HistoryError::BindingConflict);
            }
            Ok(())
        })?;
        evidence
            .members
            .first()
            .ok_or(HistoryError::EvidenceMismatch)
    }
}

fn check_distinct_bindings(bindings: &[BindingClaim]) -> Result<(), HistoryError> {
    bindings.iter().try_fold(
        (BTreeSet::new(), BTreeSet::new()),
        |(mut ids, mut sources), binding| {
            if !ids.insert(binding.id.as_str()) {
                return Err(HistoryError::Duplicate("binding ID"));
            }
            if !sources.insert(&binding.source) {
                return Err(HistoryError::SourceReuse);
            }
            Ok((ids, sources))
        },
    )?;
    Ok(())
}
