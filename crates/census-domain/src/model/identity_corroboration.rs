use super::identity_attestation::IdentityLineage;
use super::identity_decision::person_key;
use super::identity_index::{IdentityFact, PersonKey};
use super::{
    AppliedIdentityKind, AthleteCandidateId, AthleteIdentityIndex, CanonicalAthlete, Gender,
    IdentityAttestation, IdentityError,
};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn attested_documents(
    athlete: &CanonicalAthlete,
    attested: &[IdentityAttestation],
) -> Result<BTreeMap<PersonKey, Vec<IdentityLineage>>, IdentityError> {
    athlete
        .identity_attestations
        .iter()
        .chain(attested)
        .try_fold(BTreeMap::new(), |mut documents, claim| {
            include_claim(athlete, claim, &mut documents)?;
            Ok(documents)
        })
}

fn include_claim(
    athlete: &CanonicalAthlete,
    claim: &IdentityAttestation,
    documents: &mut BTreeMap<PersonKey, Vec<IdentityLineage>>,
) -> Result<(), IdentityError> {
    let Some(key) = bound_key(athlete, claim)? else {
        return Ok(());
    };
    let claims = documents.entry(key).or_default();
    if !claims.iter().any(|lineage| lineage.matches(claim)) {
        if let Some(lineage) = claim.independent_lineage() {
            claims.push(lineage);
        }
    }
    Ok(())
}

fn bound_key(
    athlete: &CanonicalAthlete,
    claim: &IdentityAttestation,
) -> Result<Option<PersonKey>, IdentityError> {
    claim.validate()?;
    if !athlete.identities().any(|identity| {
        identity.namespace == claim.subject.namespace && identity.id == claim.subject.id
    }) {
        return Err(IdentityError::UnboundAttestation(claim.subject.id.clone()));
    }
    Ok(person_key(&claim.subject))
}

pub(super) fn member_facts<'a>(
    index: &'a AthleteIdentityIndex,
    ids: &BTreeSet<&AthleteCandidateId>,
) -> Vec<&'a IdentityFact> {
    ids.iter()
        .filter_map(|id| index.facts.get(id.as_str()))
        .collect()
}

pub(super) fn positive_identity(kind: AppliedIdentityKind, facts: &[&IdentityFact]) -> bool {
    match kind {
        AppliedIdentityKind::SourceBound | AppliedIdentityKind::DifferentPerson => false,
        AppliedIdentityKind::SamePerson => same_person_evidence(facts),
    }
}

fn same_person_evidence(facts: &[&IdentityFact]) -> bool {
    let Some(first) = facts.first() else {
        return false;
    };
    let all_parsed = facts.iter().all(|fact| fact.parsed);
    let all_same_grad_year = facts.iter().all(|fact| fact.grad_year == first.grad_year);
    let has_boys = facts.iter().any(|fact| fact.gender == Gender::Boys);
    let has_girls = facts.iter().any(|fact| fact.gender == Gender::Girls);
    if !all_parsed || !all_same_grad_year || (has_boys && has_girls) {
        return false;
    }
    if facts.iter().all(|fact| first.shares_primary(fact)) {
        return true;
    }
    if disjoint_provider_objects(facts) {
        return false;
    }
    facts
        .iter()
        .filter_map(|fact| fact.primary)
        .any(|key| corroborated_by(facts, key))
}

pub(super) fn disjoint_provider_objects(facts: &[&IdentityFact]) -> bool {
    facts
        .iter()
        .filter_map(|fact| fact.primary)
        .try_fold(BTreeMap::new(), |mut objects, primary| {
            match objects.entry(primary.0) {
                Entry::Vacant(slot) => {
                    slot.insert(primary.1);
                }
                Entry::Occupied(slot) if *slot.get() == primary.1 => {}
                Entry::Occupied(_) => return Err(()),
            }
            Ok(objects)
        })
        .is_err()
}

fn corroborated_by(facts: &[&IdentityFact], key: PersonKey) -> bool {
    if facts
        .iter()
        .any(|fact| fact.primary != Some(key) && !fact.links.contains(&key))
    {
        return false;
    }
    facts.iter().enumerate().all(|(position, fact)| {
        facts
            .iter()
            .skip(position.saturating_add(1))
            .all(|other| independent_claims(fact, other, key))
    })
}

fn independent_claims(first: &IdentityFact, second: &IdentityFact, key: PersonKey) -> bool {
    match (first.attested.get(&key), second.attested.get(&key)) {
        (Some(left), Some(right)) => left
            .iter()
            .any(|claim| right.iter().any(|other| claim.is_independent_of(other))),
        _ => false,
    }
}

impl AthleteIdentityIndex {
    pub fn positive_identity_evidence(&self, members: &[AthleteCandidateId]) -> bool {
        let ids: BTreeSet<_> = members.iter().collect();
        let facts = member_facts(self, &ids);
        ids.len() > 1
            && facts.len() == ids.len()
            && positive_identity(AppliedIdentityKind::SamePerson, &facts)
    }
}
