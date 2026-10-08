use super::super::{
    identity_corroboration::disjoint_provider_objects, identity_index::IdentityFact,
};
use super::{
    gender_bit, AthleteCandidateId, AthleteIdentityIndex, IdentityAliases, IdentityError,
    IdentityProjectionBuilder, IdentityStatus,
};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

type Components<'a> = BTreeMap<&'a AthleteCandidateId, Vec<&'a IdentityFact>>;

impl IdentityProjectionBuilder<'_> {
    pub(super) fn conflicting_roots(
        &self,
        aliases: &BTreeMap<AthleteCandidateId, AthleteCandidateId>,
    ) -> Result<BTreeSet<AthleteCandidateId>, IdentityError> {
        let mut conflicts = self.negative_conflicts()?;
        for (root, facts) in self.components(aliases)? {
            if component_conflicts(&facts) {
                conflicts.insert(root.clone());
            }
        }
        Ok(conflicts)
    }

    fn negative_conflicts(&self) -> Result<BTreeSet<AthleteCandidateId>, IdentityError> {
        let mut conflicts = BTreeSet::new();
        for distinct in &self.different {
            let mut seen = BTreeSet::new();
            for member in distinct {
                let root = self.aliases.root(member)?;
                if !seen.insert(root) {
                    conflicts.insert(root.clone());
                }
            }
        }
        Ok(conflicts)
    }

    fn components<'a>(
        &'a self,
        aliases: &'a BTreeMap<AthleteCandidateId, AthleteCandidateId>,
    ) -> Result<Components<'a>, IdentityError> {
        let mut components = BTreeMap::new();
        for (member, root) in aliases {
            let child = self.fact(member)?;
            let facts = match components.entry(root) {
                Entry::Vacant(slot) => slot.insert(vec![self.fact(root)?]),
                Entry::Occupied(slot) => slot.into_mut(),
            };
            facts.push(child);
        }
        Ok(components)
    }

    fn fact(&self, id: &AthleteCandidateId) -> Result<&IdentityFact, IdentityError> {
        self.index
            .facts
            .get(id.as_str())
            .ok_or_else(|| IdentityError::UnknownSubject(id.to_string()))
    }
}

fn component_conflicts(facts: &[&IdentityFact]) -> bool {
    let Some(first) = facts.first() else {
        return false;
    };
    disjoint_provider_objects(facts)
        || facts.iter().any(|fact| fact.grad_year != first.grad_year)
        || facts
            .iter()
            .fold(0, |genders, fact| genders | gender_bit(fact.gender))
            == 3
}

pub(super) fn finish_statuses(
    index: AthleteIdentityIndex,
    aliases: &IdentityAliases,
    conflicts: &BTreeSet<AthleteCandidateId>,
) -> Result<BTreeMap<AthleteCandidateId, IdentityStatus>, IdentityError> {
    index
        .into_facts()
        .into_iter()
        .map(|(id, fact)| {
            let root = aliases.root(&id)?;
            let status = projected_status(&fact, root, conflicts);
            Ok((id, status))
        })
        .collect()
}

fn projected_status(
    fact: &IdentityFact,
    root: &AthleteCandidateId,
    conflicts: &BTreeSet<AthleteCandidateId>,
) -> IdentityStatus {
    if conflicts.contains(root) {
        IdentityStatus::RetainedConflict
    } else if fact.status > IdentityStatus::Verified {
        fact.status
    } else if fact.authorized {
        IdentityStatus::Verified
    } else {
        IdentityStatus::Unverified
    }
}
