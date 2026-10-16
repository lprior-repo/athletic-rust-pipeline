use std::collections::BTreeSet;

use super::{HistoryError, HistoryProjection, SplitRequest};

impl HistoryProjection {
    pub(super) fn check_split(
        &self,
        split: &SplitRequest,
        affected: &BTreeSet<String>,
    ) -> Result<(), HistoryError> {
        if split.partitions.len() < 2 {
            return Err(HistoryError::EvidenceMismatch);
        }
        self.check_correction(
            &split.evidence,
            affected,
            &split.revoke_merges,
            &split.affected_record_keys,
        )?;
        self.check_partition(split, affected)?;
        split.partitions.iter().try_for_each(|partition| {
            partition.binding_ids.iter().try_for_each(|id| {
                if let Some(membership) = self.memberships.get(id) {
                    if !partition
                        .binding_ids
                        .contains(&membership.claim.anchor_binding_id)
                    {
                        return Err(HistoryError::Invalid("source epoch evidence required"));
                    }
                }
                Ok(())
            })
        })?;
        let mut work = 0;
        self.check_revocations(split, affected, &mut work)?;
        split.partitions.iter().try_for_each(|partition| {
            let bindings = partition.binding_ids.iter().cloned().collect();
            if !self.connected_after(&bindings, &split.revoke_merges, &mut work)? {
                return Err(HistoryError::Partition);
            }
            Ok(())
        })
    }

    fn check_partition(
        &self,
        split: &SplitRequest,
        affected: &BTreeSet<String>,
    ) -> Result<(), HistoryError> {
        let assigned =
            split
                .partitions
                .iter()
                .try_fold(BTreeSet::new(), |mut seen, partition| {
                    let represented = partition.binding_ids.iter().any(|id| {
                        self.bindings
                            .get(id)
                            .is_some_and(|entry| entry.claim.candidate == partition.representative)
                    });
                    if partition.binding_ids.is_empty() || !represented {
                        return Err(HistoryError::Partition);
                    }
                    partition.binding_ids.iter().try_for_each(|binding| {
                        if !seen.insert(binding.clone()) {
                            return Err(HistoryError::Partition);
                        }
                        Ok(())
                    })?;
                    Ok(seen)
                })?;
        let assigned =
            split
                .unresolved_bindings
                .iter()
                .try_fold(assigned, |mut seen, binding| {
                    if !seen.insert(binding.clone()) {
                        return Err(HistoryError::Partition);
                    }
                    Ok(seen)
                })?;
        if &assigned != affected {
            return Err(HistoryError::Partition);
        }
        Ok(())
    }

    fn check_revocations(
        &self,
        split: &SplitRequest,
        affected: &BTreeSet<String>,
        work: &mut usize,
    ) -> Result<(), HistoryError> {
        self.check_revoked_edges(&split.revoke_merges, affected, work)?;
        self.merges.iter().try_for_each(|(sequence, merge)| {
            let steps = split
                .partitions
                .len()
                .checked_add(1)
                .and_then(|count| merge.binding_ids.len().checked_mul(count))
                .and_then(|steps| steps.checked_add(affected.len()))
                .and_then(|steps| steps.checked_add(1))
                .ok_or(HistoryError::CounterOverflow)?;
            self.charge_graph_work(steps, work)?;
            if merge.revoked_at.is_some()
                || split.revoke_merges.contains(sequence)
                || merge.binding_ids.is_disjoint(affected)
            {
                return Ok(());
            }
            let retained = split.partitions.iter().any(|partition| {
                merge
                    .binding_ids
                    .iter()
                    .all(|id| partition.binding_ids.contains(id))
            });
            if !retained {
                return Err(HistoryError::SurvivingMerge);
            }
            Ok(())
        })
    }
}
