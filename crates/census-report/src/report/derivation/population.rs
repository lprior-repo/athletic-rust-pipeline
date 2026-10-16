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
        Self::population_with_aliases(
            dataset,
            scope,
            grad_year,
            jurisdictions,
            &dataset.canonical_aliases,
        )
    }

    pub(super) fn selected_population(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[census_domain::UsJurisdiction],
    ) -> crate::report::ReportResult<Self> {
        let identities = dataset.verified_public_identities()?;
        let mut selected = Self::population_with_aliases(
            dataset,
            scope,
            grad_year,
            jurisdictions,
            identities.aliases(),
        );
        selection::retain(&mut selected.athletes, |athlete| {
            identities.public_id(athlete.id.as_str()).is_some()
        });
        Ok(selected)
    }

    fn population_with_aliases(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[census_domain::UsJurisdiction],
        aliases: &'a std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            dataset,
            scope,
            grad_year,
            jurisdictions: jurisdictions.to_vec(),
            school_state: school_state_index(dataset.schools.values()),
            schools: dataset.schools.values().cloned().collect(),
            outside_schools: Vec::new(),
            athletes: borrowed_or_collapsed(&dataset.athletes, aliases),
            outside_athletes: Vec::new(),
            scoped_athletes: 0,
            meets: Cow::Borrowed(&dataset.meets),
            outside_meets: Vec::new(),
            coaches: Cow::Borrowed(&dataset.coaches),
            outside_coaches: Vec::new(),
            coach_observations: Cow::Borrowed(&dataset.coach_observations),
            events: Cow::Borrowed(&dataset.events),
            performances: Vec::new(),
            athlete_aliases: aliases,
            dropped_rows: 0,
            current_affiliations: super::CurrentAffiliationView::Historical,
            affiliation_exclusions: std::collections::BTreeMap::new(),
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
        if matches!(
            self.current_affiliations,
            super::CurrentAffiliationView::Selected(_)
        ) {
            self.performances.retain(|performance| {
                let subject = performance.athlete.as_str();
                let canonical = self
                    .athlete_aliases
                    .get(subject)
                    .map_or(subject, String::as_str);
                cohort.contains(canonical)
            });
        }
    }
}
