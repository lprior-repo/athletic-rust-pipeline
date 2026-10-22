use std::collections::{BTreeMap, BTreeSet};

use crate::model::{AthleteCandidateId, StoreIdentity};

use super::{
    budget, genesis, record, request_order, BindingClaim, EstablishmentOrdinal, HistoryError,
    HistoryRecord, HistoryRequest, PublicAthleteId, RegistryHead, ReplayLimits, SourceObjectKey,
};

#[cfg(test)]
mod index_tests;

#[derive(Debug, Clone)]
pub(super) enum IdentityState {
    Current,
    Redirect {
        target: PublicAthleteId,
    },
    Ambiguous {
        successors: Vec<PublicAthleteId>,
        unresolved: bool,
    },
}

#[derive(Debug, Clone)]
pub(super) struct IdentityEntry {
    pub ordinal: EstablishmentOrdinal,
    pub representative: AthleteCandidateId,
    pub owned_bindings: BTreeSet<String>,
    pub state: IdentityState,
    pub revoked: Vec<u64>,
    pub changed_at: u64,
}

#[derive(Debug, Clone)]
pub(super) struct BindingEntry {
    pub claim: BindingClaim,
    pub owner: Option<PublicAthleteId>,
    pub established_at: u64,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct MergeEntry {
    pub binding_ids: BTreeSet<String>,
    pub revoked_at: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HistoryProjection {
    pub(super) installation: StoreIdentity,
    pub(super) head: RegistryHead,
    pub(super) limits: ReplayLimits,
    pub(super) text_bytes: usize,
    pub(super) identities: BTreeMap<PublicAthleteId, IdentityEntry>,
    pub(super) bindings: BTreeMap<String, BindingEntry>,
    pub(super) memberships: BTreeMap<String, super::membership::MembershipEntry>,
    // Derived from retained claims; stable IDs preserve live ownership after corrections.
    pub(super) candidate_bindings: BTreeMap<AthleteCandidateId, BTreeSet<String>>,
    pub(super) candidate_memberships: BTreeMap<AthleteCandidateId, BTreeSet<String>>,
    pub(super) source_bindings: BTreeMap<SourceObjectKey, BTreeSet<String>>,
    pub(super) withheld: BTreeMap<u64, Vec<String>>,
    pub(super) merges: BTreeMap<u64, MergeEntry>,
    pub(super) applications: BTreeSet<String>,
    pub(super) affected: BTreeMap<u64, Vec<String>>,
}

pub fn replay_candidate_history(
    installation: &StoreIdentity,
    expected_head: &RegistryHead,
    records: &[HistoryRecord],
    limits: ReplayLimits,
) -> Result<HistoryProjection, HistoryError> {
    if records.len() > limits.records {
        return Err(HistoryError::Limit("records"));
    }
    let initial = HistoryProjection::empty(installation, limits)?;
    let projection = records
        .iter()
        .try_fold(initial, |state, record| state.extend_candidate(record))?;
    if &projection.head != expected_head {
        return Err(HistoryError::Head);
    }
    Ok(projection)
}

impl HistoryProjection {
    fn empty(installation: &StoreIdentity, limits: ReplayLimits) -> Result<Self, HistoryError> {
        if limits.traversal_steps == 0 {
            return Err(HistoryError::Limit("traversal steps"));
        }
        Ok(Self {
            installation: installation.clone(),
            head: genesis(installation)?,
            limits,
            text_bytes: 0,
            identities: BTreeMap::new(),
            bindings: BTreeMap::new(),
            memberships: BTreeMap::new(),
            candidate_bindings: BTreeMap::new(),
            candidate_memberships: BTreeMap::new(),
            source_bindings: BTreeMap::new(),
            withheld: BTreeMap::new(),
            merges: BTreeMap::new(),
            applications: BTreeSet::new(),
            affected: BTreeMap::new(),
        })
    }

    pub fn head(&self) -> &RegistryHead {
        &self.head
    }

    pub fn public_ids(&self) -> impl Iterator<Item = &PublicAthleteId> {
        self.identities.keys()
    }

    pub fn correction_history(&self, id: &PublicAthleteId) -> Option<&[u64]> {
        self.identities
            .get(id)
            .map(|entry| entry.revoked.as_slice())
    }

    pub fn extend_candidate(mut self, record: &HistoryRecord) -> Result<Self, HistoryError> {
        self.check_record(record)?;
        self.apply_request(&record.request, record.sequence)?;
        self.head = RegistryHead {
            sequence: record.sequence,
            digest: record.digest.clone(),
        };
        Ok(self)
    }

    pub fn revoked_merges(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.merges
            .iter()
            .filter_map(|(sequence, merge)| merge.revoked_at.map(|at| (*sequence, at)))
    }

    pub fn affected_record_keys(&self, correction_sequence: u64) -> Option<&[String]> {
        self.affected.get(&correction_sequence).map(Vec::as_slice)
    }

    fn check_record(&mut self, record: &HistoryRecord) -> Result<(), HistoryError> {
        if record.version != record::HISTORY_VERSION {
            return Err(HistoryError::Version);
        }
        if record.installation != self.installation {
            return Err(HistoryError::Installation);
        }
        if record.sequence
            != self
                .head
                .sequence
                .checked_add(1)
                .ok_or(HistoryError::CounterOverflow)?
        {
            return Err(HistoryError::Sequence);
        }
        if record.previous != self.head {
            return Err(HistoryError::Head);
        }
        if usize::try_from(record.sequence).map_err(|_| HistoryError::CounterOverflow)?
            > self.limits.records
        {
            return Err(HistoryError::Limit("records"));
        }
        self.check_request_budget(&record.request)?;
        request_order::require_canonical(&record.request)?;
        if record::payload_digest(record)? != record.digest {
            return Err(HistoryError::Payload);
        }
        Ok(())
    }

    fn check_request_budget(&mut self, request: &HistoryRequest) -> Result<(), HistoryError> {
        let bytes = budget::request(request, self.limits)?;
        self.text_bytes = self
            .text_bytes
            .checked_add(bytes)
            .ok_or(HistoryError::CounterOverflow)?;
        if self.text_bytes > self.limits.total_text_bytes {
            return Err(HistoryError::Limit("total text bytes"));
        }
        let identities = self
            .identities
            .len()
            .checked_add(record::births(request))
            .ok_or(HistoryError::CounterOverflow)?;
        if identities > self.limits.identities {
            return Err(HistoryError::Limit("identities"));
        }
        if let HistoryRequest::EstablishMatchedGroup { bindings, .. } = request {
            let total = self
                .bindings
                .len()
                .checked_add(self.memberships.len())
                .and_then(|count| count.checked_add(bindings.len()))
                .ok_or(HistoryError::CounterOverflow)?;
            if total > self.limits.bindings {
                return Err(HistoryError::Limit("bindings"));
            }
        }
        Ok(())
    }

    fn apply_request(
        &mut self,
        request: &HistoryRequest,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        let reference = match request {
            HistoryRequest::Establish { evidence, .. }
            | HistoryRequest::EstablishMatchedGroup { evidence, .. }
            | HistoryRequest::Bind { evidence, .. }
            | HistoryRequest::BindCandidateMembership { evidence, .. }
            | HistoryRequest::Merge { evidence, .. } => evidence,
            HistoryRequest::Split(split) => &split.evidence,
            HistoryRequest::Revoke(revocation) => &revocation.evidence,
        };
        if !self.applications.insert(reference.application_id.clone()) {
            return Err(HistoryError::Duplicate("application"));
        }
        match request {
            HistoryRequest::Establish { binding, evidence } => {
                self.establish(binding, evidence, sequence)
            }
            HistoryRequest::EstablishMatchedGroup { bindings, evidence } => {
                self.establish_matched_group(bindings, evidence, sequence)
            }
            HistoryRequest::Bind {
                owner,
                binding,
                evidence,
            } => self.bind(owner, binding, evidence, sequence),
            HistoryRequest::BindCandidateMembership {
                owner,
                membership,
                evidence,
            } => self.bind_candidate_membership(owner, membership, evidence, sequence),
            HistoryRequest::Merge {
                canonical,
                members,
                evidence,
            } => self.merge(canonical, members, evidence, sequence),
            HistoryRequest::Split(split) => self.split(split, sequence),
            HistoryRequest::Revoke(revocation) => self.revoke(revocation, sequence),
        }
    }

    pub(super) fn require_current(
        &self,
        id: &PublicAthleteId,
    ) -> Result<&IdentityEntry, HistoryError> {
        let entry = self
            .identities
            .get(id)
            .ok_or(HistoryError::Unknown("public ID"))?;
        if !matches!(entry.state, IdentityState::Current) {
            return Err(HistoryError::NonCanonicalMember);
        }
        Ok(entry)
    }
}
