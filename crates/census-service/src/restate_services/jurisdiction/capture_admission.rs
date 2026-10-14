use crate::restate_services::source_selection::SourceSelection;
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState};
use crate::restate_services::{
    results_arms::{self, ResultsArm},
    CaptureExecution,
};
use census_domain::model::{
    serialized_digest, AcquisitionDigest, CaptureCacheIntent, CensusRun, GradYear,
    ResultsCaptureScopeV1, RunManifest, SelectedHistoryV1, SourceAdmissionV1,
};
use census_reconcile::identity::WorkflowIdentity;
use census_store::{CaptureWriterV1, ResultsCaptureOwnerV1, Store};
use restate_sdk::prelude::{HandlerError, TerminalError};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub(super) enum CaptureAdmissionError {
    #[error("capture run manifest is missing")]
    RunManifestMissing,
    #[error("capture run, cohort or store identity differs from the retained manifest")]
    RunMismatch,
    #[error("capture jurisdiction is outside the retained manifest")]
    JurisdictionOutsideManifest,
    #[error("capture owner does not match the original JurisdictionCensus object")]
    OwnerKeyMismatch,
    #[error("capture history differs from the bound owner window")]
    HistoryConflict,
    #[error("capture source plan is missing or its selected role is unsupported")]
    RoleUnsupported,
    #[error(transparent)]
    Canonical(#[from] census_domain::model::CanonicalJsonError),
    #[error(transparent)]
    Domain(#[from] census_domain::model::CaptureDomainError),
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
}

mod grants;
pub(in crate::restate_services) use grants::GrantedTargets;

pub(in crate::restate_services) enum ResultsCaptureExecution {
    LegacyFresh,
    UnadoptedSources,
    FreshQualificationV1 {
        writer: Arc<CaptureWriterV1>,
        owner: ResultsCaptureOwnerV1,
        grants: GrantedTargets,
    },
}

pub(super) fn handler_error(error: CaptureAdmissionError) -> HandlerError {
    TerminalError::new(error.to_string()).into()
}

pub(super) fn require_bound_manifest(
    execution: &CaptureExecution,
    manifest: Option<&RunManifest>,
    request: &JurisdictionRequest,
) -> Result<(), CaptureAdmissionError> {
    let CaptureExecution::FreshQualificationV1(writer) = execution else {
        return Ok(());
    };
    let manifest = manifest.ok_or(CaptureAdmissionError::RunManifestMissing)?;
    if manifest.store_identity != writer.store_identity().as_str()
        || manifest.run.season() != request.season
        || manifest.run.revision() != request.revision.get()
        || manifest.cohort != GradYear::CO2027
    {
        return Err(CaptureAdmissionError::RunMismatch);
    }
    if !manifest.jurisdictions.contains(&request.jurisdiction) {
        return Err(CaptureAdmissionError::JurisdictionOutsideManifest);
    }
    Ok(())
}

pub(super) fn admit_results_owner(
    store: &Store,
    execution: &CaptureExecution,
    actual_key: &str,
    request: &JurisdictionRequest,
    state: &JurisdictionState,
    selected: (u16, SourceSelection),
) -> Result<ResultsCaptureExecution, CaptureAdmissionError> {
    let CaptureExecution::FreshQualificationV1(writer) = execution else {
        return Ok(ResultsCaptureExecution::LegacyFresh);
    };
    let identity =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
    if actual_key != identity.as_str() || state.identity != actual_key {
        return Err(CaptureAdmissionError::OwnerKeyMismatch);
    }
    if state.history_window != Some(request.history)
        || !request.history.years().contains(&selected.0)
    {
        return Err(CaptureAdmissionError::HistoryConflict);
    }
    let manifest = store.run_manifest()?;
    require_bound_manifest(execution, manifest.as_ref(), request)?;
    let manifest = manifest.ok_or(CaptureAdmissionError::RunManifestMissing)?;
    let plan = state
        .plan
        .as_ref()
        .ok_or(CaptureAdmissionError::RoleUnsupported)?;
    let selected_milesplit = selected.1.iter().any(|source| source.slug == "milesplit");
    if !selected_milesplit {
        return Ok(ResultsCaptureExecution::UnadoptedSources);
    }
    if !plan.sweepable.iter().any(|slug| slug == "milesplit")
        || results_arms::arm_for("milesplit") != Some(ResultsArm::MilesplitResults)
    {
        return Err(CaptureAdmissionError::RoleUnsupported);
    }
    let scope = scope(
        actual_key,
        request,
        &manifest,
        selected.0,
        &plan.fingerprint,
    )?;
    let owner = writer.results_owner(store, scope)?;
    Ok(ResultsCaptureExecution::FreshQualificationV1 {
        writer: Arc::clone(writer),
        owner,
        grants: GrantedTargets::default(),
    })
}

fn scope(
    key: &str,
    request: &JurisdictionRequest,
    manifest: &RunManifest,
    year: u16,
    fingerprint: &str,
) -> Result<ResultsCaptureScopeV1, CaptureAdmissionError> {
    let run = CensusRun::new(request.season, request.revision.get())
        .ok_or(CaptureAdmissionError::RunMismatch)?;
    let digest = AcquisitionDigest::parse(&serialized_digest(manifest)?)?;
    Ok(ResultsCaptureScopeV1::new(
        key,
        request.jurisdiction,
        run,
        digest,
        SelectedHistoryV1::new(request.history, year)?,
        SourceAdmissionV1::milesplit_results(fingerprint)?,
        if request.refresh {
            CaptureCacheIntent::AcquireNew
        } else {
            CaptureCacheIntent::ReuseEligible
        },
    )?)
}

#[cfg(test)]
mod tests;
