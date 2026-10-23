use super::public_identity::{output::AcceptedIdentityFields, VerifiedPublicIdentityView};
use super::{ExportDataset, VerifiedCurrentAffiliations};
use crate::report::ReportResult;
use census_domain::model::{CanonicalSchool, SchoolId};
use census_store::{StoreError, StoreResult};
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) struct SelectedSchools<'a> {
    identities: &'a VerifiedPublicIdentityView,
    affiliations: &'a VerifiedCurrentAffiliations,
    schools: &'a BTreeMap<SchoolId, CanonicalSchool>,
}

#[derive(Clone, Copy, Serialize)]
pub(crate) struct CurrentSchoolFields<'a> {
    pub(crate) current_school_id: &'a str,
    pub(crate) current_school: &'a str,
    pub(crate) current_state: &'a str,
    pub(crate) current_school_as_of: &'a str,
}

impl<'a> SelectedSchools<'a> {
    pub(crate) fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        Ok(Self {
            identities: dataset.verified_public_identities()?,
            affiliations: dataset.verified_current_affiliations()?,
            schools: &dataset.schools,
        })
    }

    pub(crate) fn identity(&self, candidate: &str) -> StoreResult<AcceptedIdentityFields<'a>> {
        AcceptedIdentityFields::for_candidate(self.identities, candidate)
    }

    pub(crate) fn has_unresolved_current_school(&self, candidate: &str) -> ReportResult<bool> {
        let identity = match self.identity(candidate) {
            Ok(identity) => identity,
            Err(_) => return Ok(false),
        };
        let canonical = identity.canonical_candidate_id().cast();
        Ok(self.affiliations.resolution(&canonical).school().is_none())
    }

    pub(crate) fn fields(&self, candidate: &str) -> StoreResult<CurrentSchoolFields<'a>> {
        let identity = self.identity(candidate)?;
        let canonical = identity.canonical_candidate_id().cast();
        let school_id = self
            .affiliations
            .resolution(&canonical)
            .school()
            .ok_or_else(|| {
                invalid("selected best has no resolved current school at the pinned cutoff")
            })?;
        let school = self.schools.get(school_id).ok_or_else(|| {
            invalid("selected best current school is absent from the frozen input")
        })?;
        let state = school
            .state
            .ok_or_else(|| invalid("selected best current school jurisdiction is unknown"))?;
        Ok(CurrentSchoolFields {
            current_school_id: school.id.as_str(),
            current_school: &school.name,
            current_state: state.code(),
            current_school_as_of: self.affiliations.as_of().as_str(),
        })
    }
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: detail.into(),
    }
}

#[cfg(test)]
pub(crate) mod fixture;
