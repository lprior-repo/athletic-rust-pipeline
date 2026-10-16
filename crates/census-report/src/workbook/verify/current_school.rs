use crate::export::public_identity::VerifiedPublicIdentityView;
use crate::export::{ExportDataset, VerifiedCurrentAffiliations};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{CanonicalSchool, SchoolId};
use std::collections::BTreeMap;

pub(in crate::workbook) struct AffiliationExpectations<'a> {
    identities: &'a VerifiedPublicIdentityView,
    affiliations: &'a VerifiedCurrentAffiliations,
    schools: &'a BTreeMap<SchoolId, CanonicalSchool>,
}

impl<'a> AffiliationExpectations<'a> {
    pub(in crate::workbook) fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        Ok(Self {
            identities: dataset.verified_public_identities()?,
            affiliations: dataset.verified_current_affiliations()?,
            schools: &dataset.schools,
        })
    }

    pub(in crate::workbook) fn values(&self, candidate: &str) -> ReportResult<[&'a str; 4]> {
        let group = self.identities.group(candidate).ok_or_else(|| {
            invalid("affiliation readback subject is absent from accepted identity grouping")
        })?;
        if group.public_id().is_none() {
            return Err(invalid(
                "affiliation readback subject has no accepted public identity",
            ));
        }
        let canonical = group.canonical_candidate().cast();
        let id = self
            .affiliations
            .resolution(&canonical)
            .school()
            .ok_or_else(|| {
                invalid("affiliation readback subject has unresolved current affiliation")
            })?;
        let school = self.schools.get(id).ok_or_else(|| {
            invalid("affiliation readback current school is absent from frozen schools")
        })?;
        let jurisdiction = school.state.ok_or_else(|| {
            invalid("affiliation readback current school has unknown jurisdiction")
        })?;
        Ok([
            id.as_str(),
            &school.name,
            jurisdiction.code(),
            self.affiliations.as_of().as_str(),
        ])
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}
