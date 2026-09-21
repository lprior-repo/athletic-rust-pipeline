use super::{AssessmentWire, DetailRow};
use crate::{
    domain::{
        evidence::EvidenceRef,
        identity::{AthleteId, EvidenceDigest},
    },
    runtime::{
        acquisition::ProfileAcquisition,
        row_protocol::{AcceptanceMethod, RowReport},
    },
};
use anyhow::{bail, Result};
use std::collections::HashSet;

mod acceptance;
mod identity;

use acceptance::{
    candidate_is_corroborated, verify_deterministic, verify_local_authorization,
    verify_local_review, verify_participation,
};
use identity::verify_identity;

pub(super) fn verify_profiles(profiles: &[ProfileAcquisition]) -> Result<()> {
    profiles.iter().try_for_each(|acquisition| {
        let Some(profile) = acquisition.profile.as_ref() else {
            return Ok(());
        };
        if profile.profile_url.athlete_id() != profile.athlete_id {
            bail!("profile URL athlete ID contradicts profile evidence ID");
        }
        if profile.documents.is_empty() {
            bail!("profile evidence has no retained source documents");
        }
        let response_documents = acquisition
            .responses
            .iter()
            .map(|receipt| receipt.digest.as_str())
            .collect::<HashSet<_>>();
        if profile
            .documents
            .iter()
            .any(|document| !response_documents.contains(document.as_str()))
        {
            bail!("profile document is absent from retained response receipts");
        }
        check_ref(&profile.name.evidence, &profile.documents)?;
        profile.teams.iter().try_for_each(|team| {
            if team.team_id == 0 {
                bail!("profile team has invalid ID");
            }
            check_ref(&team.name.evidence, &profile.documents)?;
            if let Some(location) = team.location.as_ref() {
                check_ref(&location.evidence, &profile.documents)?;
            }
            Ok(())
        })?;
        profile
            .graduation_years
            .iter()
            .try_for_each(|year| check_ref(&year.evidence, &profile.documents))?;
        profile
            .grades
            .iter()
            .try_for_each(|grade| check_ref(&grade.evidence, &profile.documents))?;
        profile.results.iter().try_for_each(|result| {
            if result.result_id == 0 || result.team_id == 0 || result.meet_id == 0 {
                bail!("profile result has invalid source identifiers");
            }
            check_ref(&result.evidence, &profile.documents)
        })?;
        profile.issues.iter().try_for_each(|issue| {
            if let Some(reference) = issue.evidence.as_ref() {
                check_ref(reference, &profile.documents)?;
            }
            if hard_contradiction(&issue.code) {
                bail!(
                    "profile retains hard identity contradiction: {}",
                    issue.code
                );
            }
            Ok(())
        })
    })
}

fn check_ref(reference: &EvidenceRef, documents: &[EvidenceDigest]) -> Result<()> {
    if reference.locator.trim().is_empty() {
        bail!("source evidence locator is empty");
    }
    if !documents
        .iter()
        .any(|document| document == &reference.document)
    {
        bail!("source evidence document is not retained by profile");
    }
    Ok(())
}

fn hard_contradiction(code: &str) -> bool {
    matches!(
        code,
        "identity_conflict"
            | "html_identity_mismatch"
            | "html_identity_inconsistency"
            | "team_conflict"
            | "team_name_inconsistency"
            | "team_location_inconsistency"
            | "identity_incomplete"
            | "profile_url_conflict"
            | "team_history_join_missing"
    )
}

pub(super) fn verify_positive(
    row: &DetailRow,
    report: &RowReport,
    assessment: &AssessmentWire,
    acquisitions: Vec<ProfileAcquisition>,
    selected: AthleteId,
    method: AcceptanceMethod,
) -> Result<()> {
    if !matches!(
        assessment.search,
        crate::domain::decision::SearchCompleteness::Complete { .. }
    ) {
        bail!("accepted result lacks complete search evidence");
    }
    if !assessment
        .candidates
        .iter()
        .any(|candidate| candidate.athlete_id() == selected)
    {
        bail!("accepted athlete was not supplied as an assessment candidate");
    }
    let acquisition = acquisitions
        .iter()
        .find(|item| item.athlete_id == selected)
        .ok_or_else(|| anyhow::anyhow!("accepted athlete lacks retained profile acquisition"))?;
    let profile = acquisition
        .profile
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("accepted athlete lacks retained profile evidence"))?;
    if !acquisition.complete || !acquisition.failures.is_empty() {
        bail!("accepted athlete profile acquisition is incomplete or failed");
    }
    verify_profiles(std::slice::from_ref(acquisition))?;
    verify_identity(row, profile)?;
    verify_participation(profile)?;
    match method {
        AcceptanceMethod::Deterministic => {
            if report.review.is_some() {
                bail!("deterministic acceptance carries a review outcome");
            }
            let corroborated = acquisitions
                .iter()
                .filter(|candidate| candidate_is_corroborated(row, candidate))
                .count();
            if corroborated != 1 {
                bail!("deterministic acceptance lacks a unique corroborated identity");
            }
            verify_deterministic(assessment, selected)?;
        }
        AcceptanceMethod::LocalReview => {
            verify_local_review(report, profile, selected)?;
            verify_local_authorization(row, assessment, acquisitions, selected)?;
        }
    }
    Ok(())
}
