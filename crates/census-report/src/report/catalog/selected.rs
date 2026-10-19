use super::{CanonicalEvent, EventDisposition, ExportDataset, RecruitingCatalogV1, Sport};
use crate::report::{ReportError, ReportResult};
use census_domain::model::QueryProfileV1;
use census_domain::profile_execution::{
    validate_retained_selected_qualification_profile, validate_selected_qualification_profile_v1,
    validate_selected_qualification_profile_v2,
};

#[derive(Clone, Copy)]
pub(super) enum Selection<'a> {
    Historical,
    Selected(&'a QueryProfileV1),
}

impl<'a> Selection<'a> {
    pub(super) fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        let Some(profile_id) = dataset.selected_profile_id() else {
            return Ok(Self::Historical);
        };
        dataset.ensure_input_binding()?;
        if dataset.is_historical_selected_replay() {
            return Ok(Self::Historical);
        }
        let profile = dataset.selected_authority()?.profile();
        if profile.id().map_err(census_store::StoreError::from)? != *profile_id {
            return Err(ReportError::Invariant {
                detail: "event selection profile differs from frozen authority".into(),
            });
        }
        let checked = if dataset.uses_historical_identity_contract() {
            validate_selected_qualification_profile_v1(profile).map(|_| ())
        } else if dataset.uses_historical_participation_contract() {
            validate_selected_qualification_profile_v2(profile).map(|_| ())
        } else {
            validate_retained_selected_qualification_profile(profile, dataset.semantic_policy()?)
        };
        checked.map_err(|source| ReportError::Invariant {
            detail: format!("selected event publication contract: {source}"),
        })?;
        dataset.verified_participation()?;
        Ok(Self::Selected(profile))
    }

    pub(super) fn classify(
        self,
        catalog: &RecruitingCatalogV1,
        event: &CanonicalEvent,
        sport: Sport,
    ) -> ReportResult<EventDisposition> {
        let classified = match self {
            Self::Historical => catalog.classify(event, sport),
            Self::Selected(profile) => catalog.classify_for_profile(event, sport, profile),
        };
        classified.map_err(|source| ReportError::Catalog {
            event: event.id.clone(),
            source,
        })
    }
}
