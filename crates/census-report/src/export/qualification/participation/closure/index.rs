use super::Closure;
use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventId,
    MeetId, PerformanceId, SourceNamespace, TeamId,
};
use std::collections::{BTreeMap, BTreeSet};

type Rows<'a, K, T> = BTreeMap<&'a K, Vec<(usize, &'a T)>>;
type Owners<'a> = BTreeMap<String, BTreeMap<&'a SourceNamespace, BTreeSet<&'a str>>>;

pub(super) struct Index<'a> {
    athletes: BTreeMap<String, Vec<&'a CanonicalAthlete>>,
    performances: Rows<'a, PerformanceId, CanonicalPerformance>,
    teams: Rows<'a, TeamId, CanonicalTeam>,
    meets: Rows<'a, MeetId, CanonicalMeet>,
    events: Rows<'a, EventId, CanonicalEvent>,
    owners: Owners<'a>,
}

impl<'a> Index<'a> {
    pub(super) fn new(
        raw: &'a Closure,
        dataset: &'a ExportDataset,
        identities: &IdentityPopulation<'_>,
    ) -> Self {
        let mut athletes: BTreeMap<String, Vec<&CanonicalAthlete>> = BTreeMap::new();
        for row in &raw.athletes {
            if let Some(group) = identities.canonical_candidate(row.id.as_str()) {
                athletes.entry(group.into()).or_default().push(row);
            }
        }
        Self {
            athletes,
            performances: physical(&raw.performances, |row| &row.id),
            teams: physical(&raw.teams, |row| &row.id),
            meets: physical(&raw.meets, |row| &row.id),
            events: physical(&raw.events, |row| &row.id),
            owners: owners(dataset, identities),
        }
    }

    pub(super) fn athletes(
        &self,
        identities: &IdentityPopulation<'_>,
        candidate: &str,
    ) -> Vec<&'a CanonicalAthlete> {
        match identities
            .canonical_candidate(candidate)
            .and_then(|group| self.athletes.get(group))
        {
            Some(rows) => rows.clone(),
            None => Vec::new(),
        }
    }

    pub(super) fn performances(&self, id: &PerformanceId) -> Vec<&'a CanonicalPerformance> {
        hits(&self.performances, id)
    }

    pub(super) fn team(&self, id: &TeamId) -> Vec<&'a CanonicalTeam> {
        hits(&self.teams, id)
    }

    pub(super) fn meet(&self, id: &MeetId) -> Vec<&'a CanonicalMeet> {
        hits(&self.meets, id)
    }

    pub(super) fn event(&self, id: &EventId) -> Vec<&'a CanonicalEvent> {
        hits(&self.events, id)
    }

    pub(super) fn teams<'k>(
        &self,
        ids: impl Iterator<Item = &'k TeamId>,
    ) -> Vec<&'a CanonicalTeam> {
        related(&self.teams, ids)
    }

    pub(super) fn meets<'k>(
        &self,
        ids: impl Iterator<Item = &'k MeetId>,
    ) -> Vec<&'a CanonicalMeet> {
        related(&self.meets, ids)
    }

    pub(super) fn events<'k>(
        &self,
        ids: impl Iterator<Item = &'k EventId>,
    ) -> Vec<&'a CanonicalEvent> {
        related(&self.events, ids)
    }

    pub(super) fn source_bound(
        &self,
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
    ) -> bool {
        let Some(owner) = &performance.source_athlete else {
            return false;
        };
        identities
            .canonical_candidate(performance.athlete.as_str())
            .and_then(|group| self.owners.get(group))
            .and_then(|owners| owners.get(&owner.namespace))
            .is_some_and(|ids| ids.contains(owner.id.as_str()))
    }
}

fn physical<'a, K: Ord, T>(rows: &'a [T], key: impl Fn(&'a T) -> &'a K) -> Rows<'a, K, T> {
    let mut index: Rows<'a, K, T> = BTreeMap::new();
    for (ordinal, row) in rows.iter().enumerate() {
        index.entry(key(row)).or_default().push((ordinal, row));
    }
    index
}

fn hits<'a, K: Ord, T>(index: &Rows<'a, K, T>, id: &K) -> Vec<&'a T> {
    match index.get(id) {
        Some(rows) => rows.iter().map(|(_, row)| *row).collect(),
        None => Vec::new(),
    }
}

fn related<'a, 'k, K: Ord + 'a + 'k, T>(
    index: &Rows<'a, K, T>,
    ids: impl Iterator<Item = &'k K>,
) -> Vec<&'a T> {
    let mut rows = BTreeMap::new();
    for id in ids {
        if let Some(versions) = index.get(id) {
            rows.extend(versions.iter().map(|(ordinal, row)| (*ordinal, *row)));
        }
    }
    rows.into_values().collect()
}

fn owners<'a>(dataset: &'a ExportDataset, identities: &IdentityPopulation<'_>) -> Owners<'a> {
    let mut owners: Owners<'a> = BTreeMap::new();
    for athlete in &dataset.athletes {
        if identities.admission(athlete.id.as_str()) != IdentityAdmission::Accepted {
            continue;
        }
        if let Some(group) = identities.canonical_candidate(athlete.id.as_str()) {
            for identity in athlete.identities() {
                owners
                    .entry(group.into())
                    .or_default()
                    .entry(&identity.namespace)
                    .or_default()
                    .insert(identity.id.as_str());
            }
        }
    }
    owners
}

#[cfg(test)]
mod tests;
