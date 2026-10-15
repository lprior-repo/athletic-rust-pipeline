use super::reads::Tables;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    EventKind,
};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

type AthletePopulation<'a> = (Cow<'a, [CanonicalAthlete]>, &'a HashMap<String, String>);

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
    pub(super) fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        let (event_ids, unmapped_event_ids) = event_indices(dataset);
        let (athletes, aliases) = athlete_population(dataset)?;
        Ok(Self {
            schools: dataset.schools.values().cloned().collect(),
            coaches: &dataset.coaches,
            meets: &dataset.meets,
            athletes,
            aliases,
            performances: &dataset.performances,
            event_ids,
            unmapped_event_ids,
        })
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

pub(super) fn athlete_population(dataset: &ExportDataset) -> ReportResult<AthletePopulation<'_>> {
    let Some(_) = dataset.selected_profile_id() else {
        return Ok((
            super::super::derivation::borrowed_or_collapsed(
                &dataset.athletes,
                &dataset.canonical_aliases,
            ),
            &dataset.canonical_aliases,
        ));
    };
    let identities = dataset.verified_public_identities()?;
    let aliases = identities.aliases();
    let mut athletes = super::super::derivation::borrowed_or_collapsed(&dataset.athletes, aliases);
    let accepted = |athlete: &CanonicalAthlete| identities.public_id(athlete.id.as_str()).is_some();
    if !athletes.iter().all(accepted) {
        athletes.to_mut().retain(accepted);
    }
    Ok((athletes, aliases))
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
