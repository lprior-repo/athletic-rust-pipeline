use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{ContractRefV1, ProfileDate, ProfileId};
use census_domain::profile_execution::{
    validate_current_selected_qualification_profile, validate_selected_qualification_profile_v1,
    validate_selected_qualification_profile_v2,
};
use serde::{Deserialize, Serialize};

pub(super) const REVISION: u32 = 8;
pub(super) const HISTORICAL_REVISION: u32 = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectedProfile {
    profile_id: ProfileId,
    as_of: ProfileDate,
    participation_rule: ContractRefV1,
}

impl SelectedProfile {
    pub(super) fn of(dataset: &ExportDataset) -> ReportResult<Self> {
        let profile_id = dataset.selected_profile_id().ok_or_else(|| {
            super::invariant("selected publication requires a frozen profile binding".into())
        })?;
        let as_of = dataset.selected_as_of().ok_or_else(|| {
            super::invariant("selected publication requires a frozen profile cutoff".into())
        })?;
        let participation_rule = dataset.selected_participation_rule().ok_or_else(|| {
            super::invariant("selected publication requires a frozen participation rule".into())
        })?;
        dataset.verified_participation()?;
        dataset.verified_current_affiliations()?;
        Ok(Self {
            profile_id: profile_id.clone(),
            as_of: as_of.clone(),
            participation_rule: participation_rule.clone(),
        })
    }
}

pub(super) fn revision(dataset: &ExportDataset) -> ReportResult<u32> {
    let revision = if dataset.selected_profile_id().is_some() {
        dataset.ensure_current_selected_write()?;
        REVISION
    } else {
        super::ARTIFACT_REVISION
    };
    binding(revision, dataset)?;
    Ok(revision)
}

pub(super) fn binding(
    revision: u32,
    dataset: &ExportDataset,
) -> ReportResult<Option<SelectedProfile>> {
    super::versions::validate_policy(revision, dataset.lineage.policy_revision)?;
    match revision {
        1..=6 if dataset.selected_profile_id().is_none() => Ok(None),
        HISTORICAL_REVISION if dataset.is_historical_selected_replay() => {
            SelectedProfile::of(dataset).map(Some)
        }
        REVISION => {
            verified_profile(dataset)?;
            SelectedProfile::of(dataset).map(Some)
        }
        _ => Err(super::invariant(
            "historical publication cannot carry a selected profile binding".into(),
        )),
    }
}

pub(super) fn validate(revision: u32, binding: Option<&SelectedProfile>) -> ReportResult<()> {
    match (revision, binding) {
        (1..=6, None) | (HISTORICAL_REVISION | REVISION, Some(_)) => Ok(()),
        _ => Err(super::invariant(
            "publication revision and selected profile binding disagree".into(),
        )),
    }
}

fn verified_profile(
    dataset: &ExportDataset,
) -> ReportResult<&census_domain::model::QueryProfileV1> {
    dataset.ensure_input_binding()?;
    if dataset.is_historical_selected_replay() {
        return Err(super::invariant(
            "historical selected replay cannot authorize current publication".into(),
        ));
    }
    let profile = dataset.selected_authority()?.profile();
    let profile_id = profile.id().map_err(census_store::StoreError::from)?;
    if dataset.selected_profile_id() != Some(&profile_id)
        || dataset.selected_as_of() != Some(&profile.input().as_of)
        || dataset.selected_participation_rule() != Some(&profile.input().policies.participation)
    {
        return Err(super::invariant(
            "current selected publication differs from frozen profile".into(),
        ));
    }
    if dataset.uses_historical_identity_contract() {
        let _checked = validate_selected_qualification_profile_v1(profile).map_err(|source| {
            super::invariant(format!("unsupported historical selected profile: {source}"))
        })?;
    } else if dataset.uses_historical_participation_contract() {
        let _checked = validate_selected_qualification_profile_v2(profile).map_err(|source| {
            super::invariant(format!(
                "unsupported historical participation profile: {source}"
            ))
        })?;
    } else {
        let _checked =
            validate_current_selected_qualification_profile(profile).map_err(|source| {
                super::invariant(format!(
                    "unsupported selected publication profile: {source}"
                ))
            })?;
    }
    dataset.verified_public_identities()?;
    Ok(profile)
}

pub(in crate::workbook) fn ensure_current_options(
    dataset: &ExportDataset,
    options: &crate::workbook::Options,
) -> ReportResult<()> {
    dataset.ensure_current_selected_write()?;
    if dataset.selected_profile_id().is_none() {
        return Ok(());
    }
    ensure_options_for_revision(dataset, options, REVISION)
}

pub(in crate::workbook) fn ensure_verification_options(
    dataset: &ExportDataset,
    options: &crate::workbook::Options,
) -> ReportResult<()> {
    if dataset.selected_profile_id().is_none() {
        return Ok(());
    }
    let revision = if dataset.is_historical_selected_replay() {
        HISTORICAL_REVISION
    } else {
        REVISION
    };
    ensure_options_for_revision(dataset, options, revision)
}

pub(super) fn ensure_options_for_revision(
    dataset: &ExportDataset,
    options: &crate::workbook::Options,
    revision: u32,
) -> ReportResult<()> {
    binding(revision, dataset)?;
    if revision == REVISION {
        ensure_profile_options(verified_profile(dataset)?, options)?;
    }
    Ok(())
}

fn ensure_profile_options(
    profile: &census_domain::model::QueryProfileV1,
    options: &crate::workbook::Options,
) -> ReportResult<()> {
    let input = profile.input();
    let [cohort] = input.graduation_years.as_slice() else {
        return Err(super::invariant(
            "selected publication supports exactly one cohort".into(),
        ));
    };
    let mismatch = if options.grad_year != Some(cohort.get()) {
        Some("selected graduation year cannot be overridden by workbook options")
    } else if options.jurisdictions != input.jurisdictions {
        Some("selected jurisdictions require the exact canonical profile order")
    } else if options.school_year != input.contact_school_year {
        Some("selected contact school year cannot be overridden by workbook options")
    } else if options.scope != crate::report::Scope::AllSources {
        Some("selected publication requires all-source evidence scope")
    } else if options.limit.is_some() {
        Some("selected publication cannot limit the profile population")
    } else {
        None
    };
    match mismatch {
        Some(detail) => Err(super::invariant(detail.into())),
        None => Ok(()),
    }
}

#[cfg(test)]
pub(in crate::workbook) mod fixture;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod revision_tests;
