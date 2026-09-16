#[path = "positive.rs"]
mod positive;

use positive::verify_positive;

use super::{DetailRow, ResultVerificationReport};
use crate::{
    domain::{
        evidence::{EvidenceRef, ProfileEvidence},
        identity::{AthleteId, SourceRowKey},
    },
    runtime::{
        acquisition::ProfileAcquisition,
        protocol::{ReviewOutcome, ReviewVerdict},
        row_protocol::{RowReport, RowResolution, ROW_PROTOCOL_REVISION},
    },
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
pub(super) enum VerifiedState {
    Accepted,
    Review,
    NoMatch,
    Pending,
}

impl VerifiedState {
    pub(super) fn add_to(self, report: &mut ResultVerificationReport) -> Result<()> {
        let slot = match self {
            Self::Accepted => &mut report.accepted_rows,
            Self::Review => &mut report.review_rows,
            Self::NoMatch => &mut report.no_match_rows,
            Self::Pending => &mut report.pending_rows,
        };
        let next = (*slot)
            .checked_add(1)
            .context("result verification count overflow")?;
        *slot = next;
        Ok(())
    }
}
#[derive(Debug, Deserialize)]
struct AssessmentWire {
    decision: crate::domain::decision::Decision,
    candidates: Vec<CandidateWire>,
    search: crate::domain::decision::SearchCompleteness,
    #[serde(default)]
    verified: Option<VerifiedWire>,
}

#[derive(Debug, Deserialize)]
struct CandidateWire {
    athlete_id: AthleteId,
}

#[derive(Debug, Deserialize)]
struct VerifiedWire {
    athlete_id: AthleteId,
}

pub(super) fn verify_row(row: &DetailRow) -> Result<VerifiedState> {
    verify_source_binding(row)?;
    let Some(report) = row.report.as_ref() else {
        verify_pending(row)?;
        return Ok(VerifiedState::Pending);
    };
    verify_report_binding(row, report)?;
    let profiles = decode_profiles(row, report)?;
    match &report.resolution {
        RowResolution::Accepted { athlete_id, method } => {
            let assessment = decode_assessment(row)?;
            verify_positive(row, report, &assessment, &profiles, *athlete_id, *method)?;
            Ok(VerifiedState::Accepted)
        }
        RowResolution::CompleteSearchNoMatch => {
            if report.review.as_ref().is_some_and(is_positive_review) {
                bail!("no-match row carries a positive selection");
            }
            if let Some(assessment) = row.assessment.as_ref() {
                verify_no_match(&decode_value(assessment)?)?;
            }
            Ok(VerifiedState::NoMatch)
        }
        RowResolution::ReviewRequired => {
            if let Some(assessment) = row.assessment.as_ref() {
                verify_review_state(&decode_value(assessment)?)?;
            }
            Ok(VerifiedState::Review)
        }
    }
}

fn verify_source_binding(row: &DetailRow) -> Result<()> {
    let key = SourceRowKey::parse(&row.source.source_key).context("source key is invalid")?;
    if key.sheet() != row.source.sheet || key.row() != row.source.excel_row {
        bail!("source key does not bind source sheet and Excel row");
    }
    Ok(())
}

fn verify_pending(row: &DetailRow) -> Result<()> {
    if row.report_digest.is_some()
        || row.assessment.is_some()
        || !row.profile_artifacts.is_empty()
        || !row.performance_evidence.is_empty()
    {
        bail!("pending row carries terminal evidence");
    }
    Ok(())
}

fn verify_report_binding(row: &DetailRow, report: &RowReport) -> Result<()> {
    if row.report_digest.is_none() {
        bail!("terminal row is missing report digest");
    }
    if report.revision != ROW_PROTOCOL_REVISION {
        bail!("row report revision is unsupported");
    }
    if report.job.source.as_str() != row.source.source_key {
        bail!("row report source does not bind detail source");
    }
    if row.assessment.is_none() != report.assessment.is_none() {
        bail!("assessment presence does not match row report");
    }
    Ok(())
}

fn decode_assessment(row: &DetailRow) -> Result<AssessmentWire> {
    row.assessment
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("terminal row is missing assessment evidence"))
        .and_then(decode_value)
}

fn decode_value(value: &serde_json::Value) -> Result<AssessmentWire> {
    serde_json::from_value(value.clone()).context("decoding assessment evidence")
}

fn decode_profiles(row: &DetailRow, report: &RowReport) -> Result<Vec<ProfileAcquisition>> {
    if row.profile_artifacts.len() != report.profile_evidence.len() {
        bail!("profile artifact count does not match row report evidence references");
    }
    let profiles = row
        .profile_artifacts
        .iter()
        .enumerate()
        .map(|(index, value)| {
            serde_json::from_value::<ProfileAcquisition>(value.clone())
                .with_context(|| format!("decoding profile artifact {index}"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut ids = HashSet::new();
    profiles.iter().try_for_each(|acquisition| {
        if !ids.insert(acquisition.athlete_id) {
            bail!("duplicate athlete ID in profile artifacts");
        }
        if let Some(profile) = acquisition.profile.as_ref() {
            if profile.athlete_id != acquisition.athlete_id {
                bail!("profile evidence ID does not match acquisition ID");
            }
        }
        Ok(())
    })?;
    Ok(profiles)
}
fn verify_no_match(assessment: &AssessmentWire) -> Result<()> {
    if assessment.decision != crate::domain::decision::Decision::CompleteSearchNoMatch
        || assessment.verified.is_some()
    {
        bail!("no-match row has an inconsistent assessment");
    }
    Ok(())
}

fn verify_review_state(assessment: &AssessmentWire) -> Result<()> {
    if assessment.decision == crate::domain::decision::Decision::DeterministicAccepted
        || assessment.verified.is_some()
    {
        bail!("review row carries a positive deterministic assessment");
    }
    Ok(())
}
pub(super) fn profile_refs(profile: &ProfileEvidence) -> Vec<EvidenceRef> {
    profile
        .teams
        .iter()
        .flat_map(|team| {
            std::iter::once(team.name.evidence.clone()).chain(
                team.location
                    .iter()
                    .map(|location| location.evidence.clone()),
            )
        })
        .chain(
            profile
                .graduation_years
                .iter()
                .map(|year| year.evidence.clone()),
        )
        .chain(
            profile
                .issues
                .iter()
                .filter_map(|issue| issue.evidence.clone()),
        )
        .collect()
}
fn is_positive_review(outcome: &ReviewOutcome) -> bool {
    matches!(
        outcome,
        ReviewOutcome::Reviewed {
            verdict: ReviewVerdict::Select { .. },
            ..
        }
    )
}
