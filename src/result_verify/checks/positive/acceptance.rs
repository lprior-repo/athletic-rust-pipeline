use super::super::{profile_refs, AssessmentWire, DetailRow};
use super::identity::verify_identity;
use super::verify_profiles;
use crate::{
    domain::{
        decision::{self, Decision, ReviewChoice},
        evidence::{ProfileEvidence, ResultAttribution, Sport},
        identity::AthleteId,
    },
    runtime::{
        acquisition::ProfileAcquisition,
        protocol::{ReviewOutcome, ReviewVerdict},
        row_protocol::RowReport,
    },
};
use anyhow::{bail, Result};

pub(super) fn verify_participation(profile: &ProfileEvidence) -> Result<()> {
    let attributed = profile.results.iter().any(|result| {
        result.result_id > 0
            && matches!(result.sport, Sport::TrackField | Sport::CrossCountry)
            && match &result.attribution {
                ResultAttribution::Individual => true,
                ResultAttribution::VerifiedRelayMember { relay_athlete_id } => {
                    AthleteId::new(*relay_athlete_id).is_ok()
                }
                ResultAttribution::Unresolved { .. } => false,
            }
    });
    if !attributed {
        bail!("accepted athlete lacks attributed TF/XC participation");
    }
    Ok(())
}
pub(super) fn candidate_is_corroborated(row: &DetailRow, acquisition: &ProfileAcquisition) -> bool {
    let Some(profile) = acquisition.profile.as_ref() else {
        return false;
    };
    acquisition.complete
        && acquisition.failures.is_empty()
        && verify_profiles(std::slice::from_ref(acquisition)).is_ok()
        && verify_identity(row, profile).is_ok()
        && verify_participation(profile).is_ok()
}

pub(super) fn verify_deterministic(assessment: &AssessmentWire, selected: AthleteId) -> Result<()> {
    if !matches!(
        &assessment.search,
        crate::domain::decision::SearchCompleteness::Complete { .. }
    ) {
        bail!("deterministic acceptance lacks complete search evidence");
    }
    if assessment.decision != crate::domain::decision::Decision::DeterministicAccepted {
        bail!("deterministic acceptance has a non-deterministic assessment");
    }
    if assessment.verified.as_ref().map(|value| value.athlete_id) != Some(selected) {
        bail!("deterministic assessment verified selection does not match report");
    }
    Ok(())
}

pub(super) fn verify_local_review(
    report: &RowReport,
    profile: &ProfileEvidence,
    selected: AthleteId,
) -> Result<()> {
    let Some(ReviewOutcome::Reviewed {
        verdict:
            ReviewVerdict::Select {
                athlete_id,
                evidence,
                ..
            },
        ..
    }) = report.review.as_ref()
    else {
        bail!("local acceptance missing retained review outcome prerequisite");
    };
    if *athlete_id != selected || evidence.is_empty() {
        bail!("local review selection does not bind selected athlete");
    }
    let allowed = profile_refs(profile);
    if evidence
        .iter()
        .any(|reference| !allowed.iter().any(|item| item == reference))
    {
        bail!("local review cites unsupported profile evidence");
    }
    Ok(())
}

pub(super) fn verify_local_authorization(
    row: &DetailRow,
    assessment: &AssessmentWire,
    acquisitions: Vec<ProfileAcquisition>,
    selected: AthleteId,
) -> Result<()> {
    if assessment.decision != Decision::IdentityReview {
        bail!("local acceptance lacks an identity-review assessment");
    }
    if acquisitions
        .iter()
        .any(|acquisition| !acquisition.complete || !acquisition.failures.is_empty())
    {
        bail!("local acceptance contains incomplete profile discovery");
    }
    let profiles = acquisitions
        .into_iter()
        .map(|acquisition| {
            acquisition
                .profile
                .ok_or_else(|| anyhow::anyhow!("local acceptance lacks a candidate profile"))
        })
        .collect::<Result<Vec<_>>>()?;
    let recomputed = decision::assess(
        &row.source,
        profiles
            .iter()
            .map(crate::domain::candidate::CandidateEvidence::Complete),
        assessment.search.clone(),
    )?;
    if recomputed.decision() != Decision::IdentityReview {
        bail!("retained source evidence does not require identity review");
    }
    let authorized = decision::apply_review(&recomputed, ReviewChoice::Select(selected))?;
    if authorized.accepted_athlete_id() != Some(selected) {
        bail!("retained source evidence does not authorize the local selection");
    }
    Ok(())
}
