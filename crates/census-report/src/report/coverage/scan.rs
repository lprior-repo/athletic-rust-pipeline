use super::reads::Tables;
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    EventKind,
};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

pub(super) struct Scanned<'a> {
    pub(super) schools: Vec<CanonicalSchool>,
    pub(super) coaches: &'a [CanonicalCoach],
    pub(super) meets: &'a [CanonicalMeet],
    pub(super) athletes: Cow<'a, [CanonicalAthlete]>,
    pub(super) aliases: &'a HashMap<String, String>,
    pub(super) performances: &'a [CanonicalPerformance],
    pub(super) event_ids: HashSet<String>,
    pub(super) unmapped_event_ids: HashSet<String>,
}

impl<'a> Scanned<'a> {
    pub(super) fn of(dataset: &'a ExportDataset) -> Self {
        let (event_ids, unmapped_event_ids) = event_indices(dataset);
        Self {
            schools: dataset.schools.values().cloned().collect(),
            coaches: &dataset.coaches,
            meets: &dataset.meets,
            athletes: super::super::derivation::borrowed_or_collapsed(
                &dataset.athletes,
                &dataset.canonical_aliases,
            ),
            aliases: &dataset.canonical_aliases,
            performances: &dataset.performances,
            event_ids,
            unmapped_event_ids,
        }
    }

    pub(super) fn tables(&self) -> Tables<'_> {
        Tables {
            schools: &self.schools,
            athletes: &self.athletes,
            aliases: self.aliases,
            coaches: self.coaches,
            meets: self.meets,
            performances: self.performances,
        }
    }
}

fn event_indices(dataset: &ExportDataset) -> (HashSet<String>, HashSet<String>) {
    let event_ids = dataset
        .events
        .iter()
        .map(|event| event.id.as_str().to_string())
        .collect();
    let unmapped = dataset
        .events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unmapped { .. }))
        .map(|event| event.id.as_str().to_string())
        .collect();
    (event_ids, unmapped)
}
