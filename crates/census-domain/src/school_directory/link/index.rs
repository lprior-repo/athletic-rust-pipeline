use std::collections::BTreeMap;

use crate::UsJurisdiction;

use super::super::address::PostalAddress;
use super::super::entry::SchoolDirectoryEntry;
use super::super::key::{DirectoryKey, IdentifiedKey};
use super::super::label::SourceLabel;
use super::super::name::MatchForm;
use super::attest::{corroborated, record_label};
use super::forms::{candidate_forms, census_is_middle, is_middle, name_variants};
use super::{
    AttestedRecord, CandidateRef, DirectoryIndex, Form, IndexEntry, LinkDecision, LinkMatch,
    ReviewReason, MAX_CANDIDATES,
};

enum Hit {
    Linked(Box<LinkMatch>),
    Ambiguous(Vec<CandidateRef>),
}

impl DirectoryIndex {
    pub fn build(entries: &[SchoolDirectoryEntry]) -> Self {
        let mut index = Self {
            entries: Vec::with_capacity(entries.len()),
            by_name: BTreeMap::new(),
            by_core: BTreeMap::new(),
        };
        entries.iter().for_each(|entry| index.insert_entry(entry));
        index
    }

    fn insert_entry(&mut self, entry: &SchoolDirectoryEntry) {
        let Some((form, indexed)) = index_entry(entry) else {
            return;
        };
        let Ok(position) = u32::try_from(self.entries.len()) else {
            return;
        };
        self.entries.push(indexed);
        if let Some(name) = entry.name() {
            name_variants(name.as_str(), &form)
                .into_iter()
                .for_each(|variant| {
                    self.by_core.entry(variant).or_default().push(position);
                });
        }
        self.by_name.entry(form).or_default().push(position);
    }

    pub fn link(
        &self,
        school_name: &str,
        normalized_name: &str,
        city: Option<&str>,
        state: UsJurisdiction,
        aliases: &[String],
        attested: &[AttestedRecord],
    ) -> LinkDecision {
        let primary = MatchForm::of(school_name);
        let normalized = MatchForm::of(normalized_name);
        let middle = census_is_middle(&primary) || census_is_middle(&normalized);
        let city_form = city.map(MatchForm::of).filter(|form| !form.is_empty());
        let forms = candidate_forms(school_name, &primary, &normalized, aliases);
        let mut ambiguous: Option<Vec<CandidateRef>> = None;
        for form in &forms {
            let candidates: Vec<&IndexEntry> = self
                .select(&form.form, state, middle)
                .into_iter()
                .filter(|entry| corroborated(entry, city_form.as_ref(), attested))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let preferred = prefer_city(candidates, city_form.as_ref());
            match hit(form, &preferred) {
                Some(Hit::Linked(matched)) => return LinkDecision::Linked(matched),
                Some(Hit::Ambiguous(distinct)) => {
                    let _ = ambiguous.get_or_insert(distinct);
                }
                None => {}
            }
        }
        if let Some(candidates) = ambiguous {
            return LinkDecision::Review {
                reason: ReviewReason::Ambiguous,
                candidates,
            };
        }
        LinkDecision::NoMatch
    }

    pub fn link_name(
        &self,
        name: &str,
        city: Option<&str>,
        state: UsJurisdiction,
        attested: &[AttestedRecord],
    ) -> LinkDecision {
        self.link(name, name, city, state, &[], attested)
    }

    fn select<'a>(
        &'a self,
        form: &MatchForm,
        state: UsJurisdiction,
        middle: bool,
    ) -> Vec<&'a IndexEntry> {
        let mut out: Vec<&IndexEntry> = Vec::new();
        for map in [&self.by_name, &self.by_core] {
            let Some(positions) = map.get(form) else {
                continue;
            };
            for &position in positions {
                let Some(entry) = usize::try_from(position)
                    .ok()
                    .and_then(|index| self.entries.get(index))
                else {
                    continue;
                };
                if entry.state != state || entry.middle != middle {
                    continue;
                }
                out.push(entry);
            }
        }
        out
    }
}

fn hit(form: &Form, preferred: &[&IndexEntry]) -> Option<Hit> {
    let distinct = distinct_candidates(preferred);
    if let [single] = distinct.as_slice() {
        let first = preferred.first().copied()?;
        return Some(Hit::Linked(Box::new(LinkMatch {
            key: single.key.clone(),
            source: single.source.clone(),
            address: first.address.clone(),
            website: first.website.clone(),
            rule: form.rule,
            matched_form: form.form.as_str().to_string(),
        })));
    }
    Some(Hit::Ambiguous(distinct))
}

fn identity(entry: &SchoolDirectoryEntry) -> Option<(IdentifiedKey, SourceLabel)> {
    let key = match entry.key() {
        DirectoryKey::Nces(id) => IdentifiedKey::Nces(id.clone()),
        DirectoryKey::Pss(id) => IdentifiedKey::Pss(id.clone()),
        DirectoryKey::StateRecord { state, id } => {
            let label = record_label(entry.sources(), *state)?;
            return Some((
                IdentifiedKey::StateRecord {
                    state: *state,
                    id: id.clone(),
                },
                label,
            ));
        }
        DirectoryKey::Weak(_) => return None,
    };
    let source = if entry.sources().contains(&SourceLabel::Ccd) {
        SourceLabel::Ccd
    } else if entry.sources().contains(&SourceLabel::Pss) {
        SourceLabel::Pss
    } else {
        return None;
    };
    Some((key, source))
}

fn entry_state(key: &IdentifiedKey, entry: &SchoolDirectoryEntry) -> Option<UsJurisdiction> {
    let jurisdiction = entry.jurisdiction();
    let address_state = entry.address().and_then(PostalAddress::state);
    match key {
        IdentifiedKey::StateRecord { state, .. } => match address_state {
            Some(published) if published != *state => None,
            _ => Some(*state),
        },
        _ => jurisdiction.or(address_state),
    }
}

fn index_entry(entry: &SchoolDirectoryEntry) -> Option<(MatchForm, IndexEntry)> {
    let (key, source) = identity(entry)?;
    let name = entry.name()?;
    let form = MatchForm::of(name.as_str());
    if form.is_empty() {
        return None;
    }
    let address = entry.address();
    let state = entry_state(&key, entry)?;
    let city = address
        .and_then(PostalAddress::city)
        .map(|city| MatchForm::of(city.as_str()))
        .filter(|form| !form.is_empty());
    let postal = retained_address(&key, address);
    let indexed = IndexEntry {
        key,
        source,
        city,
        state,
        address: postal,
        website: entry.website().map(|website| website.as_str().to_string()),
        middle: is_middle(&form, entry.grades()),
    };
    Some((form, indexed))
}

fn retained_address(key: &IdentifiedKey, address: Option<&PostalAddress>) -> Option<PostalAddress> {
    match key {
        IdentifiedKey::Nces(_) | IdentifiedKey::Pss(_) => address.cloned(),
        IdentifiedKey::StateRecord { .. } => {
            address.filter(|address| address.line1().is_some()).cloned()
        }
    }
}

fn prefer_city<'a>(
    candidates: Vec<&'a IndexEntry>,
    city: Option<&MatchForm>,
) -> Vec<&'a IndexEntry> {
    let Some(city) = city else {
        return candidates;
    };
    let filtered: Vec<&IndexEntry> = candidates
        .iter()
        .copied()
        .filter(|entry| entry.city.as_ref() == Some(city))
        .collect();
    if filtered.is_empty() {
        candidates
    } else {
        filtered
    }
}

fn distinct_candidates(candidates: &[&IndexEntry]) -> Vec<CandidateRef> {
    let mut out: Vec<CandidateRef> = Vec::new();
    for entry in candidates {
        if out.iter().any(|candidate| candidate.key == entry.key) {
            continue;
        }
        if out.len() >= MAX_CANDIDATES {
            break;
        }
        out.push(CandidateRef {
            key: entry.key.clone(),
            source: entry.source.clone(),
        });
    }
    out
}
