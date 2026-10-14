use std::collections::{BTreeSet, VecDeque};
use std::ops::ControlFlow;

use super::replay::{IdentityEntry, IdentityState};
use super::{
    HistoryError, HistoryProjection, PublicAthleteId, PublicResolution, ResolvedSuccessor,
};

struct Walk {
    pending: VecDeque<PublicAthleteId>,
    visited: BTreeSet<PublicAthleteId>,
    resolved: BTreeSet<PublicAthleteId>,
    ambiguous: bool,
    unresolved: bool,
    changed_at: u64,
    revoked: BTreeSet<u64>,
}

impl HistoryProjection {
    pub fn resolve_public(&self, id: &PublicAthleteId) -> Result<PublicResolution, HistoryError> {
        if !self.identities.contains_key(id) {
            return Ok(PublicResolution::Unknown);
        }
        let walk = Walk {
            pending: VecDeque::from([id.clone()]),
            visited: BTreeSet::new(),
            resolved: BTreeSet::new(),
            ambiguous: false,
            unresolved: false,
            changed_at: 0,
            revoked: BTreeSet::new(),
        };
        let walked = (0..self.limits.traversal_steps).try_fold(walk, |walk, _| {
            if walk.pending.is_empty() {
                return ControlFlow::Break(Ok(walk));
            }
            match self.walk_step(walk) {
                Ok(walk) => ControlFlow::Continue(walk),
                Err(error) => ControlFlow::Break(Err(error)),
            }
        });
        let walk = match walked {
            ControlFlow::Continue(walk) => walk,
            ControlFlow::Break(result) => result?,
        };
        if !walk.pending.is_empty() {
            return Err(HistoryError::Limit("traversal steps"));
        }
        self.finish_walk(id, walk)
    }

    pub(super) fn root(&self, id: &PublicAthleteId) -> Result<PublicAthleteId, HistoryError> {
        match self.resolve_public(id)? {
            PublicResolution::Canonical(value)
            | PublicResolution::Redirect {
                successor: value, ..
            }
            | PublicResolution::RevokedRedirect {
                successor: value, ..
            } => Ok(value.public_id),
            PublicResolution::Ambiguous { .. } | PublicResolution::Unresolved => {
                Err(HistoryError::BindingConflict)
            }
            PublicResolution::Unknown => Err(HistoryError::Unknown("public ID")),
        }
    }

    fn walk_step(&self, mut walk: Walk) -> Result<Walk, HistoryError> {
        let Some(id) = walk.pending.pop_front() else {
            return Ok(walk);
        };
        if !walk.visited.insert(id.clone()) {
            return Ok(walk);
        }
        let entry = self
            .identities
            .get(&id)
            .ok_or(HistoryError::Unknown("redirect target"))?;
        walk.revoked.extend(&entry.revoked);
        walk.changed_at = walk.changed_at.max(entry.changed_at);
        match &entry.state {
            IdentityState::Current => {
                walk.resolved.insert(id);
            }
            IdentityState::Redirect { target } => {
                walk.pending.push_back(target.clone());
            }
            IdentityState::Ambiguous {
                successors,
                unresolved,
            } => {
                let queued = walk
                    .pending
                    .len()
                    .checked_add(successors.len())
                    .ok_or(HistoryError::CounterOverflow)?;
                if queued > self.limits.traversal_steps {
                    return Err(HistoryError::Limit("traversal steps"));
                }
                walk.pending.extend(successors.iter().cloned());
                walk.ambiguous = true;
                walk.unresolved |= unresolved;
            }
        }
        Ok(walk)
    }

    fn finish_walk(
        &self,
        requested: &PublicAthleteId,
        walk: Walk,
    ) -> Result<PublicResolution, HistoryError> {
        let successors = walk
            .resolved
            .iter()
            .map(|id| self.successor(id))
            .collect::<Result<Vec<_>, _>>()?;
        let revoked_merges: Vec<_> = walk.revoked.into_iter().collect();
        if walk.ambiguous {
            return Ok(PublicResolution::Ambiguous {
                successors,
                unresolved_bindings: walk.unresolved,
                changed_at: walk.changed_at,
                revoked_merges,
            });
        }
        let [successor] = successors.as_slice() else {
            return Err(HistoryError::Invalid("redirect graph"));
        };
        if &successor.public_id == requested {
            return Ok(PublicResolution::Canonical(successor.clone()));
        }
        if !revoked_merges.is_empty() {
            return Ok(PublicResolution::RevokedRedirect {
                successor: successor.clone(),
                changed_at: walk.changed_at,
                revoked_merges,
            });
        }
        Ok(PublicResolution::Redirect {
            successor: successor.clone(),
            changed_at: walk.changed_at,
        })
    }

    pub(super) fn successor(
        &self,
        id: &PublicAthleteId,
    ) -> Result<ResolvedSuccessor, HistoryError> {
        let IdentityEntry {
            ordinal,
            representative,
            ..
        } = self.require_current(id)?;
        Ok(ResolvedSuccessor {
            public_id: id.clone(),
            representative_candidate: representative.clone(),
            established: *ordinal,
        })
    }
}
