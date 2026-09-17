use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{
    domain::{
        candidate::{CandidateEvidence, HtmlIdentity, NameExclusion},
        decision::{assess, Assessment, SearchCompleteness},
        identity::EvidenceDigest,
    },
    model::SourceRecord,
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe},
        row_protocol::{CandidateCoverage, RowReport},
    },
};
use serde_json::Value;

pub(super) fn recompute(
    row: &Value,
    source: &SourceRecord,
    discovery: EvidenceDigest,
) -> Result<Assessment> {
    let profiles: Vec<ProfileAcquisition> =
        serde_json::from_value(row["profile_artifacts"].clone())?;
    let probes: Vec<ProfileProbe> = serde_json::from_value(row["identity_artifacts"].clone())?;
    let report: RowReport = serde_json::from_value(row["report"].clone())?;
    let exclusions = report
        .candidates
        .iter()
        .filter_map(|coverage| match coverage {
            CandidateCoverage::NameExcluded {
                athlete_id,
                source_name,
                ..
            } => Some((athlete_id, source_name)),
            _ => None,
        })
        .map(|(athlete_id, source_name)| {
            let probe = probes
                .iter()
                .find(|probe| probe.athlete_id == *athlete_id)
                .context("excluded fixture candidate has no probe")?;
            let html = probe
                .html
                .as_ref()
                .context("excluded fixture candidate has no HTML")?;
            let exclusion = NameExclusion::new(
                source_name.clone(),
                &probe.identities,
                HtmlIdentity {
                    athlete_id: html.athlete_id,
                    profile_url: &html.profile_url,
                    names: &html.identity_hints,
                    issues: &html.issues,
                    document: &html.document,
                },
            )?;
            Ok::<_, anyhow::Error>((probe, exclusion))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut evidence = profiles
        .iter()
        .map(|acquisition| {
            if !acquisition.complete || !acquisition.failures.is_empty() {
                bail!("trusted fixture contains an incomplete acquisition");
            }
            Ok(CandidateEvidence::Complete(
                acquisition
                    .profile
                    .as_ref()
                    .context("trusted fixture acquisition has no profile")?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    evidence.extend(
        exclusions
            .iter()
            .map(|(probe, exclusion)| CandidateEvidence::NameExcluded {
                profiles: &probe.profiles,
                exclusion,
            }),
    );
    assess(
        source,
        evidence,
        SearchCompleteness::Complete {
            evidence: discovery,
        },
    )
}
