use super::{AthleteCandidateId, IdentityError};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct IdentityAliases {
    parents: BTreeMap<AthleteCandidateId, AthleteCandidateId>,
}

impl IdentityAliases {
    pub(super) fn join(
        &mut self,
        left: &AthleteCandidateId,
        right: &AthleteCandidateId,
    ) -> Result<(), IdentityError> {
        let left = self.root(left)?;
        let right = self.root(right)?;
        if left == right {
            return Ok(());
        }
        let (child, parent) = if left > right {
            (left, right)
        } else {
            (right, left)
        };
        self.parents.insert(child.clone(), parent.clone());
        Ok(())
    }

    pub(super) fn root<'a>(
        &'a self,
        subject: &'a AthleteCandidateId,
    ) -> Result<&'a AthleteCandidateId, IdentityError> {
        let mut current = subject;
        for _ in 0..self.parents.len() {
            let Some(parent) = self.parents.get(current.as_str()) else {
                return Ok(current);
            };
            if parent >= current {
                return Err(IdentityError::AliasCycle);
            }
            current = parent;
        }
        if self.parents.contains_key(current.as_str()) {
            Err(IdentityError::AliasCycle)
        } else {
            Ok(current)
        }
    }

    pub(super) fn flattened(
        &self,
    ) -> Result<BTreeMap<AthleteCandidateId, AthleteCandidateId>, IdentityError> {
        let mut result = BTreeMap::new();
        for subject in self.parents.keys() {
            let root = self.root(subject)?;
            result.insert(subject.clone(), root.clone());
        }
        Ok(result)
    }
}
