use super::super::{
    acquisition::{ProfileProbe, TeamRequest},
    protocol::{DocumentReceipt, FailureCode, OperationFailure, RetryEvidence},
};
use super::{
    assembly,
    parsing::{ParsedSource, Payload},
    reporting::{failure, terminal},
    team::TeamObservation,
};
use crate::{
    domain::{
        evidence::{EvidenceIssue, ProfileEvidence},
        identity::AthleteId,
        name::BioIdentityObservation,
    },
    profile,
};
use anyhow::Result;
use restate_sdk::prelude::*;

#[derive(Debug, Default)]
pub(super) struct BuildState {
    pub(super) responses: Vec<DocumentReceipt>,
    pub(super) operations: Vec<RetryEvidence>,
    pub(super) failures: Vec<OperationFailure>,
    pub(super) profiles: Vec<ProfileEvidence>,
    pub(super) identities: Vec<BioIdentityObservation>,
    pub(super) html: Option<profile::HtmlProfileEvidence>,
    pub(super) requests: Vec<TeamRequest>,
    pub(super) issues: Vec<EvidenceIssue>,
    pub(super) complete: bool,
}

pub(super) fn profile_probe(athlete_id: AthleteId, state: BuildState) -> ProfileProbe {
    ProfileProbe {
        athlete_id,
        responses: state.responses,
        operations: state.operations,
        failures: state.failures,
        profiles: state.profiles,
        identities: state.identities,
        html: state.html,
        requests: state.requests,
        issues: state.issues,
        complete: state.complete,
    }
}

pub(super) fn state_from_probe(probe: ProfileProbe) -> BuildState {
    BuildState {
        responses: probe.responses,
        operations: probe.operations,
        failures: probe.failures,
        profiles: probe.profiles,
        identities: probe.identities,
        html: probe.html,
        requests: probe.requests,
        issues: probe.issues,
        complete: probe.complete,
    }
}

pub(super) fn absorb_initial(
    mut state: BuildState,
    item: ParsedSource,
) -> Result<BuildState, HandlerError> {
    state.responses.extend(item.responses.clone());
    if let Some(source_failure) = item.source_failure {
        state.complete = false;
        state.failures.push(source_failure);
    }
    if let Some(message) = item.failure {
        state.complete = false;
        state.failures.push(failure(
            FailureCode::MalformedResponse,
            message,
            item.responses.clone(),
        )?);
    }
    match item.payload {
        Some(Payload::Bio {
            profile,
            identity,
            requests,
            issues,
        }) => {
            state.profiles.push(*profile);
            if let Some(identity) = identity {
                state.identities.push(identity);
            } else {
                state.complete = false;
            }
            state.requests.extend(requests);
            state.issues.extend(issues);
        }
        Some(Payload::Html(html)) => {
            state.issues.extend(html.issues.clone());
            state.html = Some(html);
        }
        None => state.complete = false,
    }
    Ok(state)
}

pub(super) fn finalize_profile(
    profiles: Vec<ProfileEvidence>,
    html: Option<profile::HtmlProfileEvidence>,
    observations: Vec<TeamObservation>,
    issues: Vec<EvidenceIssue>,
    failures: &mut Vec<OperationFailure>,
    complete: &mut bool,
) -> Result<Option<ProfileEvidence>, HandlerError> {
    let assembly = assembly::assemble(profiles, html, observations, issues.clone(), *complete)
        .map_err(terminal)?;
    *complete = assembly.complete;
    if assembly.profile.is_none() && !issues.is_empty() {
        failures.push(failure(
            FailureCode::MalformedResponse,
            "profile parser issues retained without a reconciled profile".to_owned(),
            Vec::new(),
        )?);
    }
    Ok(assembly.profile)
}
