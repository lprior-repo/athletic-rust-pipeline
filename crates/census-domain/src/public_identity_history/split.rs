use std::collections::{BTreeMap, BTreeSet};

use super::replay::{IdentityEntry, IdentityState};
use super::{
    record, HistoryError, HistoryProjection, PublicAthleteId, PublicResolution, SplitRequest,
};

mod checks;

type Assignments = BTreeMap<String, PublicAthleteId>;

impl HistoryProjection {
    pub(super) fn split(
        &mut self,
        split: &SplitRequest,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        self.require_current(&split.canonical)?;
        let affected = self.component_bindings(&split.canonical)?;
        self.check_split(split, &affected)?;
        let assignments = self.split_assignments(split, sequence)?;
        let predecessors = self.split_predecessors(&split.canonical)?;
        predecessors
            .iter()
            .try_for_each(|id| self.retain_predecessor(id, split, &assignments, sequence))?;
        self.install_successors(split, &assignments, sequence)?;
        split.revoke_merges.iter().try_for_each(|merge| {
            self.merges
                .get_mut(merge)
                .ok_or(HistoryError::Unknown("revoked merge"))?
                .revoked_at = Some(sequence);
            Ok::<_, HistoryError>(())
        })?;
        self.affected
            .insert(sequence, split.affected_record_keys.clone());
        Ok(())
    }

    fn split_assignments(
        &self,
        split: &SplitRequest,
        sequence: u64,
    ) -> Result<Assignments, HistoryError> {
        split.partitions.iter().enumerate().try_fold(
            BTreeMap::new(),
            |mut found, (position, partition)| {
                let id = record::public_id(&self.installation, sequence, position)?;
                partition.binding_ids.iter().for_each(|binding| {
                    found.insert(binding.clone(), id.clone());
                });
                Ok(found)
            },
        )
    }

    pub(super) fn split_predecessors(
        &self,
        canonical: &PublicAthleteId,
    ) -> Result<Vec<PublicAthleteId>, HistoryError> {
        self.identities
            .keys()
            .try_fold(Vec::new(), |mut found, id| {
                let resolved = self.resolve_public(id)?;
                let active = match resolved {
                    PublicResolution::Canonical(value)
                    | PublicResolution::Redirect {
                        successor: value, ..
                    }
                    | PublicResolution::RevokedRedirect {
                        successor: value, ..
                    } => value.public_id == *canonical,
                    PublicResolution::Unknown
                    | PublicResolution::Unresolved
                    | PublicResolution::Ambiguous { .. } => false,
                };
                if active {
                    if found.len() >= self.limits.members_per_operation {
                        return Err(HistoryError::Limit("affected public IDs"));
                    }
                    found.try_reserve(1).map_err(|_| HistoryError::Allocation)?;
                    found.push(id.clone());
                }
                Ok(found)
            })
    }

    fn retain_predecessor(
        &mut self,
        id: &PublicAthleteId,
        split: &SplitRequest,
        assignments: &Assignments,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        let entry = self
            .identities
            .get_mut(id)
            .ok_or(HistoryError::Unknown("split predecessor"))?;
        let is_canonical = id == &split.canonical;
        let successors: BTreeSet<_> = if is_canonical {
            assignments.values().cloned().collect()
        } else {
            entry
                .owned_bindings
                .iter()
                .filter_map(|binding| assignments.get(binding).cloned())
                .collect()
        };
        let unresolved = split
            .unresolved_bindings
            .iter()
            .any(|binding| is_canonical || entry.owned_bindings.contains(binding));
        let successors: Vec<_> = successors.into_iter().collect();
        entry.revoked = retained_revocations(&entry.revoked, &split.revoke_merges);
        entry.changed_at = sequence;
        entry.state = match successors.as_slice() {
            [target] if !is_canonical && !unresolved => IdentityState::Redirect {
                target: target.clone(),
            },
            _ => IdentityState::Ambiguous {
                successors,
                unresolved,
            },
        };
        Ok(())
    }

    fn install_successors(
        &mut self,
        split: &SplitRequest,
        assignments: &Assignments,
        sequence: u64,
    ) -> Result<(), HistoryError> {
        let revoked = self
            .identities
            .get(&split.canonical)
            .ok_or(HistoryError::Unknown("split canonical"))?
            .revoked
            .clone();
        split
            .partitions
            .iter()
            .enumerate()
            .try_for_each(|(position, partition)| {
                let id = record::public_id(&self.installation, sequence, position)?;
                let entry = IdentityEntry {
                    ordinal: record::ordinal(sequence, position)?,
                    representative: partition.representative.clone(),
                    owned_bindings: partition.binding_ids.iter().cloned().collect(),
                    state: IdentityState::Current,
                    revoked: revoked.clone(),
                    changed_at: sequence,
                };
                if self.identities.insert(id, entry).is_some() {
                    return Err(HistoryError::Duplicate("successor ID"));
                }
                Ok(())
            })?;
        assignments.iter().try_for_each(|(binding, owner)| {
            self.bindings
                .get_mut(binding)
                .ok_or(HistoryError::Unknown("partition binding"))?
                .owner = Some(owner.clone());
            Ok::<_, HistoryError>(())
        })?;
        split.unresolved_bindings.iter().try_for_each(|binding| {
            self.bindings
                .get_mut(binding)
                .ok_or(HistoryError::Unknown("unresolved binding"))?
                .owner = None;
            Ok(())
        })
    }
}

pub(super) fn retained_revocations(prior: &[u64], newly_revoked: &[u64]) -> Vec<u64> {
    prior
        .iter()
        .chain(newly_revoked)
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
