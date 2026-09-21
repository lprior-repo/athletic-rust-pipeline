use super::super::DetailRow;
use super::AssessmentWire;
use crate::{
    domain::identity::SourceRowKey,
    runtime::{
        acquisition::ProfileAcquisition,
        protocol::{ReviewOutcome, ReviewVerdict},
        row_protocol::{RowReport, ROW_PROTOCOL_REVISION},
    },
};
use anyhow::{bail, Context, Result};
use std::collections::HashSet;

pub(super) fn verify_source_binding(row: &DetailRow) -> Result<()> {
    let key = SourceRowKey::parse(&row.source.source_key).context("source key is invalid")?;
    if key.sheet() != row.source.sheet || key.row() != row.source.excel_row {
        bail!("source key does not bind source sheet and Excel row");
    }
    Ok(())
}

pub(super) fn verify_report_binding(row: &DetailRow, report: &RowReport) -> Result<()> {
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

pub(super) fn verify_pending(row: &DetailRow) -> Result<()> {
    if row.report_digest.is_some()
        || row.report.is_some()
        || row.discovery.is_some()
        || !row.identity_artifacts.is_empty()
        || row.assessment.is_some()
        || !row.profile_artifacts.is_empty()
        || !row.performance_evidence.is_empty()
    {
        bail!("pending row carries terminal evidence");
    }
    Ok(())
}

pub(super) fn decode_value(value: &serde_json::Value) -> Result<AssessmentWire> {
    serde_json::from_value(value.clone()).context("decoding assessment evidence")
}

pub(super) fn decode_profiles(row: &DetailRow) -> Result<Vec<ProfileAcquisition>> {
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
pub(super) fn verify_no_match(assessment: &AssessmentWire) -> Result<()> {
    if assessment.decision != crate::domain::decision::Decision::CompleteSearchNoMatch
        || assessment.verified.is_some()
    {
        bail!("no-match row has an inconsistent assessment");
    }
    Ok(())
}

pub(super) fn verify_review_state(assessment: &AssessmentWire) -> Result<()> {
    if assessment.decision == crate::domain::decision::Decision::DeterministicAccepted
        || assessment.verified.is_some()
    {
        bail!("review row carries a positive deterministic assessment");
    }
    Ok(())
}

pub(super) fn is_positive_review(outcome: &ReviewOutcome) -> bool {
    matches!(
        outcome,
        ReviewOutcome::Reviewed {
            verdict: ReviewVerdict::Select { .. },
            ..
        }
    )
}
