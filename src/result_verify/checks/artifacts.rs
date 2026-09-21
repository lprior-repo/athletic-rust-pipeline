use super::super::DetailRow;
use super::{verify_stored_artifact, verify_value_matches, AssessmentWire};
use crate::domain::evidence::ResultEvidence;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) fn verify_assessment_artifact(
    row: &DetailRow,
    store: &crate::store::ArtifactStore,
) -> Result<()> {
    let Some(value) = row.assessment.as_ref() else {
        return Ok(());
    };
    let assessment = AssessmentWire::deserialize(value).context("decoding assessment artifact")?;
    verify_value_matches(value, &assessment, "assessment")?;
    let digest = row
        .report
        .as_ref()
        .and_then(|report| report.assessment.as_ref())
        .context("assessment artifact has no retained report reference")?;
    verify_stored_artifact(&assessment, digest, store)
}

pub(super) fn verify_profile_artifacts(row: &DetailRow, raw: &Value) -> Result<()> {
    let raw_profiles = raw
        .get("profile_artifacts")
        .and_then(Value::as_array)
        .context("profile artifacts are not an array")?;
    if raw_profiles.len() != row.profile_artifacts.len() {
        bail!("profile artifact serialization count differs");
    }
    row.profile_artifacts
        .iter()
        .zip(raw_profiles)
        .try_for_each(|(value, raw_value)| {
            let acquisition: crate::runtime::acquisition::ProfileAcquisition =
                serde_json::from_value(value.clone())
                    .context("decoding profile acquisition artifact")?;
            verify_value_matches(value, &acquisition, "profile acquisition")?;
            if value != raw_value {
                bail!("profile artifact wrapper differs from parsed value");
            }
            Ok(())
        })?;
    if row.report.is_none() && !row.profile_artifacts.is_empty() {
        bail!("pending row carries profile artifacts");
    }
    Ok(())
}

pub(super) fn verify_performance_evidence(row: &DetailRow) -> Result<()> {
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

pub(super) fn verify_typed_value<T>(
    raw: Option<&Value>,
    typed: Option<&T>,
    name: &str,
) -> Result<()>
where
    T: Serialize,
{
    match (raw, typed) {
        (Some(value), Some(artifact)) => verify_value_matches(value, artifact, name),
        (Some(Value::Null), None) => Ok(()),
        _ => bail!("{name} presence differs from typed artifact"),
    }
}
