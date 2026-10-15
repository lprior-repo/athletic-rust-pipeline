use super::{in_run_scope, Derivation};
use crate::export::{
    CurrentAffiliationResolution, CurrentAffiliationView, ExportDataset,
    VerifiedCurrentAffiliations,
};
use crate::report::{ReportError, ReportResult, Scope};
use census_domain::model::{AthleteId, CanonicalAthlete, SchoolId};
use census_domain::UsJurisdiction;
use std::borrow::Cow;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentAffiliationExclusion {
    Unresolved,
    SchoolJurisdictionUnknown,
    OutsideRequestedJurisdictions,
}

impl CurrentAffiliationExclusion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unresolved => "current_affiliation_unresolved",
            Self::SchoolJurisdictionUnknown => "current_school_jurisdiction_unknown",
            Self::OutsideRequestedJurisdictions => "current_school_outside_requested_jurisdictions",
        }
    }
}

impl<'a> Derivation<'a> {
    pub(crate) fn for_publication(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[UsJurisdiction],
    ) -> ReportResult<Self> {
        Self::of(dataset, scope, grad_year, jurisdictions)
    }

    pub(crate) fn ensure_selected_affiliation(&self) -> ReportResult<()> {
        if self.dataset.selected_profile_id().is_some()
            && matches!(
                self.current_affiliations,
                CurrentAffiliationView::Historical
            )
        {
            return Err(invalid(
                "selected publication requires verified current affiliation",
            ));
        }
        Ok(())
    }

    pub(crate) fn selected(
        dataset: &'a ExportDataset,
        scope: Scope,
        grad_year: Option<i16>,
        jurisdictions: &[UsJurisdiction],
        affiliations: &'a VerifiedCurrentAffiliations,
    ) -> ReportResult<Self> {
        if affiliations.input_generation() != dataset.lineage.input_generation {
            return Err(invalid(
                "current affiliation belongs to a different frozen input",
            ));
        }
        if dataset.selected_profile_id() != Some(affiliations.profile_id()) {
            return Err(invalid(
                "current affiliation belongs to a different selected profile",
            ));
        }
        if dataset.selected_as_of() != Some(affiliations.as_of()) {
            return Err(invalid(
                "current affiliation belongs to a different selected cutoff",
            ));
        }
        let mut selected = Self::selected_population(dataset, scope, grad_year, jurisdictions)?;
        selected.current_affiliations = CurrentAffiliationView::Selected(affiliations);
        selected.project_current_schools(affiliations)?;
        selected.retain_geography(jurisdictions);
        for athlete in &selected.outside_athletes {
            selected.affiliation_exclusions.insert(
                athlete.id.clone(),
                CurrentAffiliationExclusion::OutsideRequestedJurisdictions,
            );
        }
        selected.retain_evidence_scope();
        selected.select_cohort();
        Ok(selected)
    }

    pub(crate) const fn current_affiliations(&self) -> CurrentAffiliationView<'a> {
        self.current_affiliations
    }

    pub fn affiliation_exclusions(&self) -> &BTreeMap<AthleteId, CurrentAffiliationExclusion> {
        &self.affiliation_exclusions
    }

    pub(crate) fn current_affiliation(
        &self,
        athlete: &AthleteId,
    ) -> Option<&CurrentAffiliationResolution> {
        match self.current_affiliations {
            CurrentAffiliationView::Historical => None,
            CurrentAffiliationView::Selected(selected) => Some(selected.resolution(athlete)),
        }
    }

    pub(crate) fn current_school_in_scope(&self, athlete: &CanonicalAthlete) -> bool {
        match self.current_affiliations {
            CurrentAffiliationView::Historical => in_run_scope(
                crate::report::coverage::jurisdiction_of(
                    &self.school_state,
                    athlete.school.as_str(),
                ),
                &self.jurisdictions,
            ),
            CurrentAffiliationView::Selected(selected) => selected
                .resolution(&athlete.id)
                .school()
                .and_then(|id| self.dataset.schools.get(id))
                .and_then(|school| school.state)
                .is_some_and(|state| in_run_scope(state.into(), &self.jurisdictions)),
        }
    }

    fn project_current_schools(
        &mut self,
        affiliations: &VerifiedCurrentAffiliations,
    ) -> ReportResult<()> {
        for athlete in self.athletes.iter() {
            match affiliations.resolution(&athlete.id).school() {
                Some(id) => {
                    let school = self.dataset.schools.get(id).ok_or_else(|| {
                        invalid("resolved current school is absent from frozen input")
                    })?;
                    if school.state.is_none() {
                        self.affiliation_exclusions.insert(
                            athlete.id.clone(),
                            CurrentAffiliationExclusion::SchoolJurisdictionUnknown,
                        );
                    }
                }
                None => {
                    self.affiliation_exclusions
                        .insert(athlete.id.clone(), CurrentAffiliationExclusion::Unresolved);
                }
            }
        }
        let schools = &self.dataset.schools;
        project(&mut self.athletes, |athlete| {
            affiliations.resolution(athlete).school().filter(|id| {
                schools
                    .get(*id)
                    .is_some_and(|school| school.state.is_some())
            })
        });
        Ok(())
    }
}

fn project<'s>(
    athletes: &mut Cow<'_, [CanonicalAthlete]>,
    school: impl Fn(&AthleteId) -> Option<&'s SchoolId>,
) {
    if athletes
        .iter()
        .all(|athlete| school(&athlete.id) == Some(&athlete.school))
    {
        return;
    }
    match athletes {
        Cow::Borrowed(rows) => {
            *athletes = Cow::Owned(
                rows.iter()
                    .filter_map(|athlete| {
                        school(&athlete.id).map(|school| {
                            let mut projected = athlete.clone();
                            projected.school = school.clone();
                            projected
                        })
                    })
                    .collect(),
            );
        }
        Cow::Owned(rows) => rows.retain_mut(|athlete| match school(&athlete.id) {
            Some(school) => {
                if athlete.school != *school {
                    athlete.school = school.clone();
                }
                true
            }
            None => false,
        }),
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests;
