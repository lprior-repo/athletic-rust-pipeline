#[path = "positive.rs"]
mod positive;

use positive::verify_positive;

use super::{DetailRow, ResultVerificationReport};
use crate::{
    domain::{
        decision::CandidateReason,
        evidence::{EvidenceRef, ProfileEvidence, ResultEvidence},
        identity::{AthleteId, SourceRowKey},
    },
    runtime::{
        acquisition::ProfileAcquisition,
        protocol::{ReviewOutcome, ReviewVerdict},
        row_protocol::{RowReport, RowResolution, ROW_PROTOCOL_REVISION},
    },
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
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
#[derive(Debug, Deserialize, Serialize)]
struct AssessmentWire {
    decision: crate::domain::decision::Decision,
    candidates: Vec<CandidateReason>,
    search: crate::domain::decision::SearchCompleteness,
    #[serde(default)]
    verified: Option<VerifiedWire>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerifiedWire {
    athlete_id: AthleteId,
}

pub(super) fn verify_embedded_artifacts(row: &DetailRow, raw: &Value) -> Result<()> {
    verify_typed_value(raw.get("report"), row.report.as_ref(), "report")?;
    verify_assessment_artifact(row, raw)?;
    verify_profile_artifacts(row, raw)?;
    verify_performance_evidence(row)?;
    Ok(())
}

fn verify_assessment_artifact(row: &DetailRow, raw: &Value) -> Result<()> {
    let Some(value) = row.assessment.as_ref() else {
        return Ok(());
    };
    let assessment: AssessmentWire =
        serde_json::from_value(value.clone()).context("decoding assessment artifact")?;
    verify_value_matches(value, &assessment, "assessment")?;
    let Some(report) = row.report.as_ref() else {
        bail!("assessment artifact is present without a row report");
    };
    let digest = typed_digest(&assessment)?;
    if report.assessment.as_ref() != Some(&digest) {
        bail!("assessment digest does not match retained assessment artifact");
    }
    let _ = raw;
    Ok(())
}

fn verify_profile_artifacts(row: &DetailRow, raw: &Value) -> Result<()> {
    let raw_profiles = raw
        .get("profile_artifacts")
        .and_then(Value::as_array)
        .context("profile artifacts are not an array")?;
    if raw_profiles.len() != row.profile_artifacts.len() {
        bail!("profile artifact serialization count differs");
    }
    if let Some(report) = row.report.as_ref() {
        if report.profile_evidence.len() != row.profile_artifacts.len() {
            bail!("profile evidence reference count differs from artifacts");
        }
        row.profile_artifacts
            .iter()
            .zip(raw_profiles)
            .zip(&report.profile_evidence)
            .try_for_each(|((value, raw_value), digest)| {
                let acquisition: crate::runtime::acquisition::ProfileAcquisition =
                    serde_json::from_value(value.clone())
                        .context("decoding profile acquisition artifact")?;
                verify_value_matches(value, &acquisition, "profile acquisition")?;
                if value != raw_value {
                    bail!("profile artifact wrapper differs from parsed value");
                }
                if typed_digest(&acquisition)? != *digest {
                    bail!("profile evidence digest does not match retained artifact");
                }
                Ok(())
            })?;
    } else if !row.profile_artifacts.is_empty() {
        bail!("pending row carries profile artifacts");
    }
    Ok(())
}

fn verify_performance_evidence(row: &DetailRow) -> Result<()> {
    let expected = row
        .profile_artifacts
        .iter()
        .map(|value| {
            serde_json::from_value::<crate::runtime::acquisition::ProfileAcquisition>(value.clone())
                .context("decoding profile acquisition for performance evidence")
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flat_map(|acquisition| {
            acquisition
                .profile
                .map_or_else(Vec::new, |profile| profile.results)
        })
        .collect::<Vec<_>>();
    if expected.len() != row.performance_evidence.len() {
        bail!("performance evidence does not match retained profile results");
    }
    row.performance_evidence
        .iter()
        .zip(expected.iter())
        .try_for_each(|(value, expected_result)| {
            let actual: ResultEvidence =
                serde_json::from_value(value.clone()).context("decoding performance evidence")?;
            verify_value_matches(value, &actual, "performance evidence")?;
            if &actual != expected_result {
                bail!("performance evidence differs from retained profile result");
            }
            Ok(())
        })
}

fn verify_typed_value<T>(raw: Option<&Value>, typed: Option<&T>, name: &str) -> Result<()>
where
    T: Serialize,
{
    match (raw, typed) {
        (Some(value), Some(artifact)) => verify_value_matches(value, artifact, name),
        (Some(Value::Null), None) => Ok(()),
        _ => bail!("{name} presence differs from typed artifact"),
    }
}

fn verify_value_matches<T: Serialize>(raw: &Value, typed: &T, name: &str) -> Result<()> {
    if serde_json::to_value(typed).context("serializing typed artifact")? != *raw {
        bail!("{name} is not the exact typed serde representation");
    }
    Ok(())
}

fn typed_digest<T: Serialize>(value: &T) -> Result<crate::domain::identity::EvidenceDigest> {
    let bytes = serde_json::to_vec(value).context("serializing typed artifact for digest")?;
    let encoded = format!("{:x}", Sha256::digest(bytes));
    crate::domain::identity::EvidenceDigest::parse(&encoded).context("parsing artifact digest")
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
            verify_positive(row, report, &assessment, profiles, *athlete_id, *method)?;
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

fn verify_report_binding(row: &DetailRow, report: &RowReport) -> Result<()> {
    let digest = typed_digest(report)?;
    if row.report_digest.as_ref() != Some(&digest) {
        bail!("row report digest does not match retained report artifact");
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
