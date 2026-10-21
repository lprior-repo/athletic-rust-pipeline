use super::{refusal, team_collection};
use crate::restate_services::wire::{
    JurisdictionRequest, JurisdictionState, SourcePlan, TeamsContinuationOutcome, TeamsFailure,
    TeamsSourceOutcome, TeamsSourceRequest, TeamsStage,
};
use census_domain::model::serialized_digest;
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::prelude::*;
use std::collections::BTreeSet;

pub(super) fn eligible(state: &JurisdictionState) -> bool {
    state.consolidated.is_none()
        && matches!(
            &state.teams,
            TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. })
                if failures.iter().any(|failure| failure.source == "milesplit")
        )
}

pub(super) fn parent<'a>(
    state: &'a JurisdictionState,
    request: &JurisdictionRequest,
) -> Result<Option<(&'a str, &'a TeamsSourceOutcome)>, HandlerError> {
    if state.consolidated.is_some() {
        return Ok(None);
    }
    let TeamsStage::Failed(TeamsFailure::SourceFailures { at, failures, .. }) = &state.teams else {
        return Ok(None);
    };
    if !failures.iter().any(|failure| failure.source == "milesplit") {
        return Ok(None);
    }
    let identity =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
    if state.identity != identity.as_str() || state.history_window != Some(request.history) {
        return Err(refusal("parent identity or history changed"));
    }
    let plan = state
        .plan
        .as_ref()
        .ok_or_else(|| refusal("retained plan absent"))?;
    require_unique_inventory(plan)?;
    let mut sources = BTreeSet::new();
    if team_collection::selected_sources(&plan.sweepable)
        .any(|source| !sources.insert(source.as_str()))
        || !sources.contains("milesplit")
        || plan.refused.iter().any(|source| source.slug == "milesplit")
        || plan
            .satisfied
            .iter()
            .any(|source| source.slug == "milesplit")
    {
        return Err(refusal("retained source plan is ambiguous"));
    }
    let mut failed = BTreeSet::new();
    if failures.iter().any(|failure| {
        !sources.contains(failure.source.as_str()) || !failed.insert(failure.source.as_str())
    }) {
        return Err(refusal("retained source failure inventory changed"));
    }
    let original = failures
        .iter()
        .find(|failure| failure.source == "milesplit")
        .ok_or_else(|| refusal("original MileSplit failure absent"))?;
    Ok(Some((at, &original.outcome)))
}

fn require_unique_inventory(plan: &SourcePlan) -> Result<(), HandlerError> {
    let mut seen = BTreeSet::new();
    for slug in &plan.sweepable {
        if !seen.insert(slug.as_str()) {
            return Err(refusal("retained source plan is ambiguous"));
        }
    }
    for source in &plan.refused {
        if !seen.insert(source.slug.as_str()) {
            return Err(refusal("retained source plan is ambiguous"));
        }
    }
    for source in &plan.satisfied {
        if !seen.insert(source.slug.as_str()) {
            return Err(refusal("retained source plan is ambiguous"));
        }
    }
    Ok(())
}

pub(super) fn continuation(
    parent: &TeamsSourceOutcome,
    original: &TeamsSourceOutcome,
    current: &TeamsContinuationOutcome,
) -> Result<bool, HandlerError> {
    if current.original_outcome_digest != digest(original)? {
        return Err(refusal("continuation original outcome digest mismatch"));
    }
    if digest(parent)? == digest(&current.outcome)? {
        return Ok(false);
    }
    if digest(parent)? != digest(original)? {
        return Err(refusal(
            "parent differs from original and settled continuation",
        ));
    }
    Ok(true)
}

pub(super) fn settled(
    state: &JurisdictionState,
    source: &str,
    outcome: &TeamsSourceOutcome,
) -> Result<(), HandlerError> {
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &state.teams else {
        return Err(refusal("retained source failures absent"));
    };
    if let Some(original) = failures.iter().find(|failure| failure.source == source) {
        if digest(&original.outcome)? != digest(outcome)? {
            return Err(refusal("another source's settled outcome changed"));
        }
    } else if !matches!(outcome, TeamsSourceOutcome::Completed { .. }) {
        return Err(refusal("another source's retained success changed"));
    }
    Ok(())
}

pub(super) fn key(request: &JurisdictionRequest, source: &str, at: &str) -> String {
    super::super::team_source::key(&TeamsSourceRequest {
        jurisdiction: request.clone(),
        source: source.to_string(),
        observed_on: request
            .observed_on
            .as_deref()
            .map_or(at, |value| value)
            .to_string(),
    })
}

pub(super) fn digest(value: &(impl serde::Serialize + ?Sized)) -> Result<String, HandlerError> {
    serialized_digest(value).map_err(|error| refusal(&format!("digest: {error}")))
}
