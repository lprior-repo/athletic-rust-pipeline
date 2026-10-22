use crate::bests::{Parents, SharedSelection};
use crate::export::ExportDataset;
use crate::report::{Derivation, EventAdmission, ReportError, ReportResult};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventId,
    MeetId, PerformanceId, ProvisionalInclusionV1,
};
use std::collections::{BTreeSet, HashMap};

mod scope;

pub(super) struct Projection<'a> {
    pub(super) source: &'a ExportDataset,
    pub(super) athletes: Vec<&'a CanonicalAthlete>,
    pub(super) performances: Vec<&'a CanonicalPerformance>,
    pub(super) bests: Vec<ReportedBest<'a>>,
    parents: Parents<'a>,
    by_performance: HashMap<&'a str, &'a CanonicalPerformance>,
    by_athlete: HashMap<&'a str, Vec<&'a CanonicalPerformance>>,
    by_best: HashMap<AthleteId, Vec<usize>>,
}

pub(super) struct ReportedBest<'a> {
    pub(super) group: crate::bests::ReportedBestGroup,
    pub(super) performance: Option<&'a CanonicalPerformance>,
    pub(super) athlete: Option<&'a CanonicalAthlete>,
}

impl<'a> Projection<'a> {
    pub(super) fn of(
        dataset: &'a ExportDataset,
        derivation: &Derivation<'_>,
        admission: &EventAdmission<'_>,
        verified_selections: &[SharedSelection],
    ) -> ReportResult<Option<Self>> {
        if dataset.selected_profile_id().is_none() {
            return Ok(None);
        }
        let authority = dataset.selected_authority()?;
        let profile = authority.profile();
        if profile.input().include_provisional != ProvisionalInclusionV1::Include {
            return Ok(None);
        }
        if derivation.grad_year().is_none() {
            return Err(ReportError::Invariant {
                detail: "revision-9 projection requires the selected cohort".into(),
            });
        }
        admission.ensure_dataset(dataset)?;
        let parents = Parents::for_reported(dataset);
        let performances = scope::performances(dataset, derivation, admission, profile, &parents);
        let mut by_athlete: HashMap<&str, Vec<&CanonicalPerformance>> = HashMap::new();
        for performance in &performances {
            by_athlete
                .entry(performance.athlete.as_str())
                .or_default()
                .push(*performance);
        }
        let athletes = scope::athletes(dataset, derivation, profile, &by_athlete);
        let by_performance = performances
            .iter()
            .map(|row| (row.id.as_str(), *row))
            .collect();
        let mut projection = Self {
            source: dataset,
            athletes,
            performances,
            bests: Vec::new(),
            parents,
            by_performance,
            by_athlete,
            by_best: HashMap::new(),
        };
        projection.assemble_bests(admission, verified_selections)?;
        Ok(Some(projection))
    }

    fn assemble_bests(
        &mut self,
        admission: &EventAdmission<'_>,
        verified: &[SharedSelection],
    ) -> ReportResult<()> {
        let selected = verified
            .iter()
            .map(|selection| {
                (
                    selection.key.clone(),
                    selection.source.performance_id.clone(),
                )
            })
            .collect::<BTreeSet<_>>();
        for group in crate::bests::reported_best_groups(self.source, admission, &self.performances)?
        {
            let performance = group
                .candidate_id
                .as_ref()
                .and_then(|id| self.performance(id));
            if group.candidate_id.is_some() && performance.is_none() {
                return Err(ReportError::Invariant {
                    detail: "reported-best candidate is absent from the frozen canonical performance set".into(),
                });
            }
            if group.conflicts.is_empty()
                && group
                    .candidate_id
                    .as_ref()
                    .is_some_and(|id| selected.contains(&(group.key.clone(), id.clone())))
            {
                continue;
            }
            let athlete = self.athlete(&group.key.athlete_id);
            self.by_best
                .entry(group.key.athlete_id.clone())
                .or_default()
                .push(self.bests.len());
            self.bests.push(ReportedBest {
                group,
                performance,
                athlete,
            });
        }
        Ok(())
    }

    pub(super) fn athlete(&self, id: &AthleteId) -> Option<&'a CanonicalAthlete> {
        self.parents.athlete(id.as_str())
    }

    pub(super) fn performance(&self, id: &PerformanceId) -> Option<&'a CanonicalPerformance> {
        self.by_performance.get(id.as_str()).copied()
    }

    pub(super) fn event(&self, id: &EventId) -> Option<&'a CanonicalEvent> {
        self.parents.canonical_event(id.as_str())
    }

    pub(super) fn meet(&self, id: &MeetId) -> Option<&'a CanonicalMeet> {
        self.parents.meet(id.as_str())
    }

    pub(super) fn performances_for(&self, id: &AthleteId) -> &[&'a CanonicalPerformance] {
        self.by_athlete.get(id.as_str()).map_or(&[], Vec::as_slice)
    }

    pub(super) fn bests_for(&self, id: &AthleteId) -> impl Iterator<Item = &ReportedBest<'a>> {
        self.by_best
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|index| self.bests.get(*index))
    }
}
