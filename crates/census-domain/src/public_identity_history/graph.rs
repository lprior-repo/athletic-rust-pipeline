use std::collections::BTreeSet;
use std::ops::ControlFlow;

use super::{HistoryError, HistoryProjection};

impl HistoryProjection {
    pub(super) fn charge_graph_work(
        &self,
        steps: usize,
        work: &mut usize,
    ) -> Result<(), HistoryError> {
        *work = work
            .checked_add(steps)
            .ok_or(HistoryError::CounterOverflow)?;
        if *work > self.limits.graph_steps {
            return Err(HistoryError::Limit("graph steps"));
        }
        Ok(())
    }

    pub(super) fn connected_after(
        &self,
        bindings: &BTreeSet<String>,
        revoked: &[u64],
        work: &mut usize,
    ) -> Result<bool, HistoryError> {
        let Some(first) = bindings.first() else {
            return Err(HistoryError::Partition);
        };
        let initial = BTreeSet::from([first.clone()]);
        let walked = (0..bindings.len()).try_fold(initial, |seen, _| {
            if seen.len() == bindings.len() {
                return ControlFlow::Break(Ok(true));
            }
            match self.expand_connections(&seen, bindings, revoked, work) {
                Ok(expanded) if expanded == seen => ControlFlow::Break(Ok(false)),
                Ok(expanded) => ControlFlow::Continue(expanded),
                Err(error) => ControlFlow::Break(Err(error)),
            }
        });
        match walked {
            ControlFlow::Break(result) => result,
            ControlFlow::Continue(seen) => Ok(seen.len() == bindings.len()),
        }
    }

    fn expand_connections(
        &self,
        seen: &BTreeSet<String>,
        bindings: &BTreeSet<String>,
        revoked: &[u64],
        work: &mut usize,
    ) -> Result<BTreeSet<String>, HistoryError> {
        self.merges
            .iter()
            .try_fold(seen.clone(), |mut expanded, (sequence, merge)| {
                let steps = merge
                    .binding_ids
                    .len()
                    .checked_mul(3)
                    .and_then(|steps| steps.checked_add(bindings.len()))
                    .and_then(|steps| steps.checked_add(1))
                    .ok_or(HistoryError::CounterOverflow)?;
                self.charge_graph_work(steps, work)?;
                if merge.revoked_at.is_none()
                    && !revoked.contains(sequence)
                    && merge.binding_ids.is_subset(bindings)
                    && !merge.binding_ids.is_disjoint(&expanded)
                {
                    expanded.extend(merge.binding_ids.iter().cloned());
                }
                Ok(expanded)
            })
    }
}
