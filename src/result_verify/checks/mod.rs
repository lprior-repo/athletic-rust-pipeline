mod artifacts;
mod positive;
mod row;

use positive::verify_positive;

use artifacts::{
    verify_assessment_artifact, verify_performance_evidence, verify_profile_artifacts,
    verify_typed_value,
};
use row::{
    decode_profiles, decode_value, is_positive_review, verify_no_match, verify_pending,
    verify_report_binding, verify_review_state, verify_source_binding,
};

use super::{DetailRow, ResultVerificationReport};
use crate::{
    domain::{
        decision::CandidateReason,
        evidence::{EvidenceRef, ProfileEvidence},
        identity::AthleteId,
    },
    runtime::row_protocol::RowResolution,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
pub(super) struct AssessmentWire {
    decision: crate::domain::decision::Decision,
    pub(super) candidates: Vec<CandidateReason>,
    pub(super) search: crate::domain::decision::SearchCompleteness,
    #[serde(default)]
    verified: Option<VerifiedWire>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerifiedWire {
    athlete_id: AthleteId,
}

pub(super) fn verify_embedded_artifacts(
    row: &DetailRow,
    raw: &Value,
    store: &crate::store::ArtifactStore,
) -> Result<()> {
    verify_typed_value(raw.get("report"), row.report.as_ref(), "report")?;
    if let Some(report) = &row.report {
        let digest = row
            .report_digest
            .as_ref()
            .context("row report digest is missing")?;
        verify_stored_artifact(report, digest, store)?;
    }
    verify_assessment_artifact(row, store)?;
    verify_profile_artifacts(row, raw)?;
    verify_performance_evidence(row)?;
    super::coverage::verify(row, store)?;
    Ok(())
}

pub(super) fn verify_value_matches<T: Serialize>(raw: &Value, typed: &T, name: &str) -> Result<()> {
    if serde_json::to_value(typed).context("serializing typed artifact")? != *raw {
        bail!("{name} is not the exact typed serde representation");
    }
    Ok(())
}

pub(super) fn verify_stored_artifact<T: Serialize>(
    value: &T,
    digest: &crate::domain::identity::EvidenceDigest,
    store: &crate::store::ArtifactStore,
) -> Result<()> {
    let retained = store
        .get_bytes(digest)
        .context("reading retained typed artifact")?;
    let embedded = serde_json::to_vec(value).context("serializing embedded typed artifact")?;
    if retained != embedded {
        bail!("embedded typed artifact differs from retained hash-verified bytes");
    }
    Ok(())
}

pub(super) fn verify_row(row: &DetailRow) -> Result<VerifiedState> {
    verify_source_binding(row)?;
    let Some(report) = row.report.as_ref() else {
        verify_pending(row)?;
        return Ok(VerifiedState::Pending);
    };
    verify_report_binding(row, report)?;
    let profiles = decode_profiles(row)?;
    let assessment = row.assessment.as_ref().map(decode_value).transpose()?;
    if let Some(assessment) = assessment.as_ref() {
        super::assessment::verify(row, &profiles, assessment)?;
    }
    match &report.resolution {
        RowResolution::Accepted { athlete_id, method } => {
            super::coverage::verify_selected(report, *athlete_id)?;
            let assessment = assessment
                .as_ref()
                .context("accepted row has no assessment")?;
            verify_positive(row, report, assessment, profiles, *athlete_id, *method)?;
            Ok(VerifiedState::Accepted)
        }
        RowResolution::CompleteSearchNoMatch => {
            if report.review.as_ref().is_some_and(is_positive_review) {
                bail!("no-match row carries a positive selection");
            }
            if let Some(assessment) = assessment.as_ref() {
                verify_no_match(assessment)?;
            }
            Ok(VerifiedState::NoMatch)
        }
        RowResolution::ReviewRequired => {
            if let Some(assessment) = assessment.as_ref() {
                verify_review_state(assessment)?;
            }
            Ok(VerifiedState::Review)
        }
    }
}

pub(super) fn decode_assessment(row: &DetailRow) -> Result<AssessmentWire> {
    row.assessment
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("terminal row is missing assessment evidence"))
        .and_then(decode_value)
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
