use super::{is_core_evidenced, Scope};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

mod aliases;
mod cohort;
mod current_affiliation;
mod historical;
#[cfg(test)]
mod participation_reducer_fixture;
mod population;
mod selection;
use crate::export::CurrentAffiliationView;
pub(super) use aliases::{borrowed_or_collapsed, collapse_athletes};
pub(crate) use cohort::candidates as cohort_candidates;
pub use current_affiliation::CurrentAffiliationExclusion;

pub(crate) fn in_run_scope(bucket: JurisdictionBucket, jurisdictions: &[UsJurisdiction]) -> bool {
    match bucket.jurisdiction() {
        None => true,
        Some(jurisdiction) => {
            jurisdiction.is_in_census_scope()
                && (jurisdictions.is_empty() || jurisdictions.contains(&jurisdiction))
        }
    }
}

pub(crate) fn exclude_out_of_scope<T>(
    rows: &mut Vec<T>,
    place: impl Fn(&T) -> JurisdictionBucket,
    jurisdictions: &[UsJurisdiction],
) -> Vec<T> {
    let (kept, excluded): (Vec<T>, Vec<T>) = rows
        .drain(..)
        .partition(|row| in_run_scope(place(row), jurisdictions));
    *rows = kept;
    excluded
}

pub struct Derivation<'a> {
    dataset: &'a ExportDataset,
    scope: Scope,
    grad_year: Option<i16>,
    jurisdictions: Vec<UsJurisdiction>,
    school_state: HashMap<&'a str, Option<UsJurisdiction>>,
    schools: Vec<CanonicalSchool>,
    outside_schools: Vec<CanonicalSchool>,
    athletes: Cow<'a, [CanonicalAthlete]>,
    outside_athletes: Vec<CanonicalAthlete>,
    scoped_athletes: usize,
    meets: Cow<'a, [CanonicalMeet]>,
    outside_meets: Vec<CanonicalMeet>,
    coaches: Cow<'a, [CanonicalCoach]>,
    outside_coaches: Vec<CanonicalCoach>,
    coach_observations: Cow<'a, [CanonicalCoach]>,
    events: Cow<'a, [CanonicalEvent]>,
    performances: Vec<&'a CanonicalPerformance>,
    athlete_aliases: &'a HashMap<String, String>,
    dropped_rows: usize,
    current_affiliations: CurrentAffiliationView<'a>,
    affiliation_exclusions:
        std::collections::BTreeMap<census_domain::model::AthleteId, CurrentAffiliationExclusion>,
}

impl<'a> Derivation<'a> {
    pub fn of(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[UsJurisdiction],
    ) -> super::ReportResult<Self> {
        if dataset.selected_profile_id().is_some() {
            dataset.verified_public_identities()?;
            dataset.verified_participation()?;
            return Self::selected(
                dataset,
                scope,
                grad_year,
                jurisdictions,
                dataset.verified_current_affiliations()?,
            );
        }
        Ok(Self::historical(dataset, scope, grad_year, jurisdictions))
    }

    fn historical(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[UsJurisdiction],
    ) -> Self {
        let mut derivation = Self::population(dataset, scope, grad_year, jurisdictions);
        derivation.retain_geography(jurisdictions);
        derivation.retain_evidence_scope();
        derivation.select_cohort();
        derivation
    }

    pub(crate) fn dataset(&self) -> &'a ExportDataset {
        self.dataset
    }

    pub(crate) const fn scope(&self) -> Scope {
        self.scope
    }

    pub(crate) const fn grad_year(&self) -> Option<i16> {
        self.grad_year
    }
    pub(crate) fn jurisdictions(&self) -> &[UsJurisdiction] {
        &self.jurisdictions
    }

    pub(crate) fn school_state(&self) -> &HashMap<&'a str, Option<UsJurisdiction>> {
        &self.school_state
    }

    pub fn schools(&self) -> &[CanonicalSchool] {
        &self.schools
    }

    pub(crate) fn outside_schools(&self) -> &[CanonicalSchool] {
        &self.outside_schools
    }

    pub fn athletes(&self) -> &[CanonicalAthlete] {
        &self.athletes
    }

    pub(crate) fn outside_athletes(&self) -> &[CanonicalAthlete] {
        &self.outside_athletes
    }

    pub(crate) const fn scoped_athletes(&self) -> usize {
        self.scoped_athletes
    }

    pub fn meets(&self) -> &[CanonicalMeet] {
        &self.meets
    }

    pub(crate) fn outside_meets(&self) -> &[CanonicalMeet] {
        &self.outside_meets
    }

    pub fn coaches(&self) -> &[CanonicalCoach] {
        &self.coaches
    }

    pub(crate) fn outside_coaches(&self) -> &[CanonicalCoach] {
        &self.outside_coaches
    }

    pub(crate) fn coach_observations(&self) -> &[CanonicalCoach] {
        &self.coach_observations
    }

    pub(crate) fn events(&self) -> &[CanonicalEvent] {
        &self.events
    }

    pub fn performances(&self) -> &[&'a CanonicalPerformance] {
        &self.performances
    }

    pub(crate) fn athlete_aliases(&self) -> &'a HashMap<String, String> {
        self.athlete_aliases
    }

    pub(crate) const fn dropped_rows(&self) -> usize {
        self.dropped_rows
    }
}

fn cohort_performances<'d>(
    dataset: &'d ExportDataset,
    scope: Scope,
    grad_year: Option<i16>,
    cohort: &HashSet<&str>,
    aliases: &HashMap<String, String>,
) -> Vec<&'d CanonicalPerformance> {
    let known_athletes = known_athletes(dataset, grad_year);
    dataset
        .performances
        .iter()
        .filter(|performance| scope != Scope::Core || is_core_evidenced(*performance))
        .filter(|performance| {
            let subject = performance.athlete.as_str();
            let canonical = aliases.get(subject).map_or(subject, String::as_str);
            cohort.contains(canonical)
                || known_athletes
                    .as_ref()
                    .is_some_and(|known| !known.contains(subject))
        })
        .collect()
}

fn known_athletes(dataset: &ExportDataset, grad_year: Option<i16>) -> Option<HashSet<&str>> {
    grad_year.is_none().then(|| {
        dataset
            .athletes
            .iter()
            .map(|athlete| athlete.id.as_str())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::{exclude_out_of_scope, in_run_scope};
    use census_domain::{JurisdictionBucket, UsJurisdiction};

    #[test]
    fn a_five_state_restriction_keeps_only_the_named_state() {
        let five = [
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Colorado,
        ];
        assert!(in_run_scope(
            JurisdictionBucket::from(UsJurisdiction::Michigan),
            &five
        ));
        assert!(!in_run_scope(
            JurisdictionBucket::from(UsJurisdiction::Wisconsin),
            &five
        ));
        assert!(!in_run_scope(
            JurisdictionBucket::from(UsJurisdiction::Alaska),
            &five
        ));
        assert!(in_run_scope(JurisdictionBucket::Unplaced, &five));
        assert!(in_run_scope(
            JurisdictionBucket::from(UsJurisdiction::Michigan),
            &[]
        ));
        assert!(!in_run_scope(
            JurisdictionBucket::from(UsJurisdiction::Alaska),
            &[]
        ));
    }

    #[test]
    fn exclusion_partitions_rows_on_the_named_restriction() {
        let mut rows = vec![
            UsJurisdiction::Michigan,
            UsJurisdiction::Wisconsin,
            UsJurisdiction::Alaska,
        ];
        let excluded = exclude_out_of_scope(
            &mut rows,
            |jurisdiction| JurisdictionBucket::from(*jurisdiction),
            &[UsJurisdiction::Michigan],
        );
        assert_eq!(rows, vec![UsJurisdiction::Michigan]);
        assert_eq!(
            excluded,
            vec![UsJurisdiction::Wisconsin, UsJurisdiction::Alaska]
        );
    }
}
