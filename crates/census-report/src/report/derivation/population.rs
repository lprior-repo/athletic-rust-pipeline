use super::{
    cohort, cohort_performances, collapse_athletes, exclude_out_of_scope, in_run_scope, Derivation,
};
use crate::export::ExportDataset;
use crate::report::coverage::{jurisdiction_of, school_state_index};
use crate::report::{retain_core, Scope};
use std::collections::HashSet;

impl<'a> Derivation<'a> {
    pub(super) fn population(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
    ) -> Self {
        Self {
            dataset,
            scope,
            grad_year,
            school_state: school_state_index(dataset.schools.values()),
            schools: dataset.schools.values().cloned().collect(),
            outside_schools: Vec::new(),
            athletes: collapse_athletes(&dataset.athletes, &dataset.canonical_aliases),
            outside_athletes: Vec::new(),
            scoped_athletes: 0,
            meets: dataset.meets.clone(),
            outside_meets: Vec::new(),
            coaches: dataset.coaches.clone(),
            outside_coaches: Vec::new(),
            coach_observations: dataset.coach_observations.clone(),
            events: dataset.events.clone(),
            performances: Vec::new(),
            athlete_aliases: &dataset.canonical_aliases,
            dropped_rows: 0,
        }
    }

    pub(super) fn retain_geography(&mut self) {
        let school_state = &self.school_state;
        self.outside_schools =
            exclude_out_of_scope(&mut self.schools, |school| school.state.into());
        self.outside_meets = exclude_out_of_scope(&mut self.meets, |meet| meet.state.into());
        self.outside_athletes = exclude_out_of_scope(&mut self.athletes, |athlete| {
            jurisdiction_of(school_state, athlete.school.as_str())
        });
        self.outside_coaches = exclude_out_of_scope(&mut self.coaches, |coach| {
            jurisdiction_of(school_state, coach.school.as_str())
        });
        self.coach_observations
            .retain(|coach| in_run_scope(jurisdiction_of(school_state, coach.school.as_str())));
    }

    pub(super) fn retain_evidence_scope(&mut self) {
        if self.scope == Scope::Core {
            retain_core(&mut self.events);
            self.dropped_rows =
                retain_core(&mut self.athletes).saturating_add(retain_core(&mut self.meets));
        }
    }

    pub(super) fn select_cohort(&mut self) {
        self.scoped_athletes = self.athletes.len();
        self.athletes
            .retain(|athlete| cohort::publishable(athlete, self.grad_year));
        let cohort: HashSet<&str> = self
            .athletes
            .iter()
            .map(|athlete| athlete.id.as_str())
            .collect();
        self.performances = cohort_performances(
            self.dataset,
            self.scope,
            self.grad_year,
            &cohort,
            self.athlete_aliases,
        );
    }
}
