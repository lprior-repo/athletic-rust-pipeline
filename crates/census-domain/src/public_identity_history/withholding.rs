use super::{HistoryError, HistoryProjection};
use std::collections::BTreeSet;
use std::ops::ControlFlow;

impl HistoryProjection {
    pub fn preview_withheld_memberships(
        &self,
        canonical: &super::PublicAthleteId,
        attachment_sequence: u64,
    ) -> Result<Vec<String>, HistoryError> {
        self.require_current(canonical)?;
        if !self
            .memberships
            .values()
            .any(|entry| entry.established_at == attachment_sequence)
        {
            return Err(HistoryError::Unknown("membership attachment"));
        }
        let affected = self.component_bindings(canonical)?;
        let revoked = [attachment_sequence];
        let mut work = 0;
        self.check_revoked_edges(&revoked, &affected, &mut work)?;
        self.membership_detachment(&affected, &revoked, &mut work)
    }

    pub(super) fn membership_detachment(
        &self,
        affected: &BTreeSet<String>,
        revoked: &[u64],
        work: &mut usize,
    ) -> Result<Vec<String>, HistoryError> {
        let first = affected
            .iter()
            .find(|id| self.bindings.contains_key(*id))
            .ok_or(HistoryError::Unknown("source component"))?;
        let initial = BTreeSet::from([first.clone()]);
        let walked = (0..affected.len()).try_fold(initial, |seen, _| {
            match self.expand_connections(&seen, affected, revoked, work) {
                Ok(next) if next == seen || next.len() == affected.len() => {
                    ControlFlow::Break(Ok(next))
                }
                Ok(next) => ControlFlow::Continue(next),
                Err(error) => ControlFlow::Break(Err(error)),
            }
        });
        let retained = match walked {
            ControlFlow::Break(result) => result?,
            ControlFlow::Continue(seen) => seen,
        };
        affected
            .difference(&retained)
            .try_fold(Vec::new(), |mut withheld, id| {
                if self.bindings.contains_key(id) {
                    return Err(HistoryError::SplitRequired);
                }
                if !self.memberships.contains_key(id) {
                    return Err(HistoryError::Unknown("membership claim"));
                }
                withheld
                    .try_reserve(1)
                    .map_err(|_| HistoryError::Allocation)?;
                withheld.push(id.clone());
                Ok(withheld)
            })
    }
}
