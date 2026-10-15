use super::{
    aliases::borrowed_or_collapsed, cohort, cohort_performances, exclude_out_of_scope,
    in_run_scope, selection, Derivation,
};
use crate::export::ExportDataset;
use crate::report::coverage::{jurisdiction_of, school_state_index};
use crate::report::{retain_core, Scope};
use std::borrow::Cow;
use std::collections::HashSet;

impl<'a> Derivation<'a> {
    pub(super) fn population(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[census_domain::UsJurisdiction],
    ) -> Self {
        Self {
            dataset,
            scope,
            grad_year,
            jurisdictions: jurisdictions.to_vec(),
            school_state: school_state_index(dataset.schools.values()),
            schools: dataset.schools.values().cloned().collect(),
            outside_schools: Vec::new(),
            athletes: borrowed_or_collapsed(&dataset.athletes, &dataset.canonical_aliases),
            outside_athletes: Vec::new(),
            scoped_athletes: 0,
            meets: Cow::Borrowed(&dataset.meets),
            outside_meets: Vec::new(),
            coaches: Cow::Borrowed(&dataset.coaches),
            outside_coaches: Vec::new(),
            coach_observations: Cow::Borrowed(&dataset.coach_observations),
            events: Cow::Borrowed(&dataset.events),
            performances: Vec::new(),
            athlete_aliases: &dataset.canonical_aliases,
            dropped_rows: 0,
        }
    }

    pub(super) fn retain_geography(&mut self, jurisdictions: &[census_domain::UsJurisdiction]) {
        let school_state = &self.school_state;
        self.outside_schools = exclude_out_of_scope(
            &mut self.schools,
            |school| school.state.into(),
            jurisdictions,
        );
        self.outside_meets = selection::exclude_out_of_scope(
            &mut self.meets,
            |meet| meet.state.into(),
            jurisdictions,
        );
        self.outside_athletes = selection::exclude_out_of_scope(
            &mut self.athletes,
            |athlete| jurisdiction_of(school_state, athlete.school.as_str()),
            jurisdictions,
        );
        self.outside_coaches = selection::exclude_out_of_scope(
            &mut self.coaches,
            |coach| jurisdiction_of(school_state, coach.school.as_str()),
            jurisdictions,
        );
        selection::retain(&mut self.coach_observations, |coach| {
            in_run_scope(
                jurisdiction_of(school_state, coach.school.as_str()),
                jurisdictions,
            )
        });
    }

    pub(super) fn retain_evidence_scope(&mut self) {
        if self.scope == Scope::Core {
            retain_core(self.events.to_mut());
            self.dropped_rows = retain_core(self.athletes.to_mut())
                .saturating_add(retain_core(self.meets.to_mut()));
        }
    }

    pub(super) fn select_cohort(&mut self) {
        self.scoped_athletes = self.athletes.len();
        selection::retain(&mut self.athletes, |athlete| {
            cohort::publishable(athlete, self.grad_year)
        });
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
