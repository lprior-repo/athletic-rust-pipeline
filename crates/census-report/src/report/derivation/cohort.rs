use super::{collapse_athletes, in_run_scope};
use crate::export::ExportDataset;
use crate::report::coverage::{in_requested_year, jurisdiction_of, school_state_index};
use crate::report::{is_core_evidenced, Scope};
use census_domain::model::{CanonicalAthlete, Confidence};

pub(super) fn publishable(athlete: &CanonicalAthlete, grad_year: Option<i16>) -> bool {
    in_requested_year(athlete, grad_year)
        && (grad_year.is_none() || athlete.derived_cohort_confidence() == Some(Confidence::HIGH))
}

pub(crate) fn candidates(
    dataset: &ExportDataset,
    scope: Scope,
    grad_year: Option<i16>,
) -> Vec<CanonicalAthlete> {
    let states = school_state_index(dataset.schools.values());
    let mut athletes = collapse_athletes(&dataset.athletes, &dataset.canonical_aliases);
    athletes.retain(|athlete| {
        in_run_scope(jurisdiction_of(&states, athlete.school.as_str()))
            && (scope != Scope::Core || is_core_evidenced(athlete))
            && in_requested_year(athlete, grad_year)
    });
    athletes
}
