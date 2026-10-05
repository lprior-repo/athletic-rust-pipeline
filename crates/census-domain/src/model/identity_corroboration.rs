use super::identity_decision::person_key;
use super::identity_index::{primary_document, IdentityFact, PersonKey};
use super::{
    AppliedIdentityKind, AthleteCandidateId, AthleteIdentityIndex, CanonicalAthlete, Gender,
    SourceIdentity,
};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn attested_documents(
    athlete: &CanonicalAthlete,
    primary: Option<PersonKey>,
    links: &[PersonKey],
    attested: &[(SourceIdentity, String)],
) -> BTreeMap<PersonKey, String> {
    let mut documents = BTreeMap::new();
    if let Some(key) = primary {
        if let Some(document) = primary_document(athlete) {
            documents.insert(key, document);
        }
    }
    for link in &athlete.source_links {
        if let Some((key, document)) = link_document(link, links) {
            documents.insert(key, document);
        }
    }
    for (identity, document) in attested {
        if let Some(key) = explicit_document(identity, document, primary, links) {
            documents.insert(key, document.clone());
        }
    }
    documents
}

fn link_document(link: &SourceIdentity, links: &[PersonKey]) -> Option<(PersonKey, String)> {
    let key = person_key(link)?;
    if !links.contains(&key) {
        return None;
    }
    link.url
        .clone()
        .filter(|url| !url.is_empty())
        .map(|url| (key, url))
}

fn explicit_document(
    identity: &SourceIdentity,
    document: &str,
    primary: Option<PersonKey>,
    links: &[PersonKey],
) -> Option<PersonKey> {
    let key = person_key(identity)?;
    if document.is_empty() || (Some(key) != primary && !links.contains(&key)) {
        return None;
    }
    Some(key)
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
    let mut objects: BTreeMap<&'static str, u64> = BTreeMap::new();
    for fact in facts {
        let Some(primary) = fact.primary else {
            continue;
        };
        match objects.entry(primary.0) {
            Entry::Vacant(slot) => {
                slot.insert(primary.1);
            }
            Entry::Occupied(slot) if *slot.get() == primary.1 => {}
            Entry::Occupied(_) => return true,
        }
    }
    false
}

fn corroborated_by(facts: &[&IdentityFact], key: PersonKey) -> bool {
    let mut documents = BTreeSet::new();
    for fact in facts {
        if fact.primary != Some(key) && !fact.links.contains(&key) {
            return false;
        }
        let Some(document) = fact.attested.get(&key) else {
            return false;
        };
        if document.is_empty() || !documents.insert(document.as_str()) {
            return false;
        }
    }
    !documents.is_empty()
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
