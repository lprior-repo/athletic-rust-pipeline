use super::VerifiedPublicIdentityView;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{AthleteIdentityProjection, IdentityStatus};
use std::sync::Arc;

pub(crate) struct IdentityPopulation<'a>(PopulationAuthority<'a>);

enum PopulationAuthority<'a> {
    Historical(Arc<AthleteIdentityProjection>),
    Selected(&'a VerifiedPublicIdentityView),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityAdmission {
    Accepted,
    Withheld,
    Absent,
}

impl<'a> IdentityPopulation<'a> {
    pub(crate) fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        if dataset.selected_profile_id().is_some() {
            return dataset
                .verified_public_identities()
                .map(|view| Self(PopulationAuthority::Selected(view)));
        }
        Ok(Self(PopulationAuthority::Historical(dataset.identities())))
    }

    pub(crate) fn selected_view(&self) -> Option<&VerifiedPublicIdentityView> {
        match &self.0 {
            PopulationAuthority::Historical(_) => None,
            PopulationAuthority::Selected(view) => Some(*view),
        }
    }

    pub(crate) fn admission(&self, candidate: &str) -> IdentityAdmission {
        match &self.0 {
            PopulationAuthority::Historical(projection) => match projection.status(candidate) {
                Ok(IdentityStatus::Verified) => IdentityAdmission::Accepted,
                Ok(_) => IdentityAdmission::Withheld,
                Err(_) => IdentityAdmission::Absent,
            },
            PopulationAuthority::Selected(view) => match view.group(candidate) {
                Some(group) if group.public_id().is_some() => IdentityAdmission::Accepted,
                Some(_) => IdentityAdmission::Withheld,
                None => IdentityAdmission::Absent,
            },
        }
    }

    pub(crate) fn canonical_candidate<'s>(&'s self, candidate: &'s str) -> Option<&'s str> {
        match &self.0 {
            PopulationAuthority::Historical(projection) => projection
                .status(candidate)
                .ok()
                .map(|_| projection.canonical_id(candidate)),
            PopulationAuthority::Selected(view) => view
                .group(candidate)
                .map(|group| group.canonical_candidate().as_str()),
        }
    }

    pub(crate) fn same_group(&self, left: &str, right: &str) -> bool {
        match (
            self.canonical_candidate(left),
            self.canonical_candidate(right),
        ) {
            (Some(left), Some(right)) => left == right,
            _ => false,
        }
    }
}
