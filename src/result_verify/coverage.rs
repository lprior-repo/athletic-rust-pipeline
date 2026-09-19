use super::{
    checks::{decode_assessment, verify_stored_artifact, verify_value_matches, AssessmentWire},
    DetailRow,
};
use crate::{
    domain::{
        candidate::CandidateCoverage as AssessedCoverage,
        decision::{CandidateReason, SearchCompleteness},
        identity::{AthleteId, EvidenceDigest},
    },
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe},
        row_protocol::{CandidateCoverage, DiscoverySummary, RowReport, RowResolution},
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn verify(row: &DetailRow, store: &ArtifactStore) -> Result<()> {
    let Some(report) = &row.report else {
        return empty_evidence(row);
    };
    let Some(discovery_digest) = &report.discovery else {
        empty_evidence(row)?;
        if !matches!(report.resolution, RowResolution::ReviewRequired)
            || !report.candidates.is_empty()
            || !report.query_evidence.is_empty()
            || report.assessment.is_some()
            || report.review.is_some()
            || report.issues.is_empty()
        {
            bail!("only explicit source-validation review may omit discovery and assessment");
        }
        return Ok(());
    };
    let discovery: DiscoverySummary = decode_bound(
        row.discovery
            .as_ref()
            .context("terminal row has no discovery artifact")?,
        discovery_digest,
        store,
    )?;
    let origin = super::source_receipts::source_origin(&report.job.snapshot, store)?;
    verify_discovery(&row.source, report, &discovery, &origin, store)?;
    let assessment = decode_assessment(row)?;
    verify_assessment(report, &discovery, &assessment)?;
    let mut probes = row.identity_artifacts.iter();
    let mut profiles = row.profile_artifacts.iter();
    report
        .candidates
        .iter()
        .zip(&assessment.candidates)
        .try_for_each(|(candidate, reason)| {
            let probe: Option<ProfileProbe> = candidate
                .probe()
                .map(|digest| {
                    decode_bound(
                        probes
                            .next()
                            .context("coverage probe artifact is missing")?,
                        digest,
                        store,
                    )
                })
                .transpose()?;
            let profile: Option<ProfileAcquisition> = candidate
                .profile()
                .map(|digest| {
                    decode_bound(
                        profiles
                            .next()
                            .context("coverage profile artifact is missing")?,
                        digest,
                        store,
                    )
                })
                .transpose()?;
            if reason.athlete_id() != candidate.athlete_id() {
                bail!("candidate assessment order differs from retained coverage");
            }
            verify_candidate(
                row,
                candidate,
                reason,
                probe.as_ref(),
                profile.as_ref(),
                &origin,
                store,
            )
        })?;
    if probes.next().is_some() || profiles.next().is_some() {
        bail!("embedded artifacts exceed candidate coverage references");
    }
    Ok(())
}

fn empty_evidence(row: &DetailRow) -> Result<()> {
    if row.discovery.is_some()
        || row.assessment.is_some()
        || !row.identity_artifacts.is_empty()
        || !row.profile_artifacts.is_empty()
        || !row.performance_evidence.is_empty()
    {
        bail!("pending or source-validation row carries candidate evidence");
    }
    Ok(())
}

fn decode_bound<T: DeserializeOwned + Serialize>(
    value: &Value,
    digest: &EvidenceDigest,
    store: &ArtifactStore,
) -> Result<T> {
    let artifact = T::deserialize(value).context("decoding typed coverage artifact")?;
    verify_value_matches(value, &artifact, "coverage artifact")?;
    verify_stored_artifact(&artifact, digest, store)?;
    Ok(artifact)
}

fn verify_discovery(
    source: &crate::model::SourceRecord,
    report: &RowReport,
    discovery: &DiscoverySummary,
    origin: &url::Url,
    store: &ArtifactStore,
) -> Result<()> {
    if discovery.job.workbook != report.job.workbook
        || discovery.job.snapshot != report.job.snapshot
        || discovery.job.source != report.job.source
        || discovery.job.rankings != report.job.rankings
        || discovery.query_artifacts != report.query_evidence
    {
        bail!("discovery is not bound to the original row job and query artifacts");
    }
    super::discovery::verify(source, discovery, origin, store)?;
    let covered = report
        .candidates
        .iter()
        .map(CandidateCoverage::athlete_id)
        .collect::<HashSet<_>>();
    if covered.len() != report.candidates.len()
        || covered.len() != discovery.candidate_ids.len()
        || !discovery
            .candidate_ids
            .iter()
            .all(|id| covered.contains(id))
    {
        bail!("discovery IDs differ from exhaustive unique candidate coverage");
    }
    if discovery.complete && !discovery.issues.is_empty() {
        bail!("complete discovery retains unresolved discovery issues");
    }
    if !matches!(report.resolution, RowResolution::ReviewRequired)
        && (!discovery.complete
            || report
                .candidates
                .iter()
                .any(|candidate| matches!(candidate, CandidateCoverage::Incomplete { .. })))
    {
        bail!("accepted or no-match row has incomplete discovery or candidate coverage");
    }
    Ok(())
}

fn verify_assessment(
    report: &RowReport,
    discovery: &DiscoverySummary,
    assessment: &AssessmentWire,
) -> Result<()> {
    let assessed = assessment
        .candidates
        .iter()
        .map(CandidateReason::athlete_id)
        .collect::<HashSet<_>>();
    if assessed.len() != assessment.candidates.len()
        || assessed.len() != discovery.candidate_ids.len()
        || !discovery
            .candidate_ids
            .iter()
            .all(|id| assessed.contains(id))
    {
        bail!("assessment IDs differ from discovery and coverage IDs");
    }
    let complete = discovery.complete
        && report
            .candidates
            .iter()
            .all(|candidate| !matches!(candidate, CandidateCoverage::Incomplete { .. }));
    match (&assessment.search, complete) {
        (SearchCompleteness::Complete { evidence }, true)
            if report.discovery.as_ref() == Some(evidence) =>
        {
            Ok(())
        }
        (SearchCompleteness::Incomplete { .. }, false) => Ok(()),
        _ => bail!("assessment search completeness differs from retained candidate coverage"),
    }
}

fn verify_candidate(
    row: &DetailRow,
    candidate: &CandidateCoverage,
    reason: &CandidateReason,
    probe: Option<&ProfileProbe>,
    profile: Option<&ProfileAcquisition>,
    origin: &url::Url,
    store: &ArtifactStore,
) -> Result<()> {
    let id = candidate.athlete_id();
    if probe.is_some_and(|value| value.athlete_id != id)
        || profile.is_some_and(|value| {
            value.athlete_id != id
                || value
                    .profile
                    .as_ref()
                    .is_some_and(|profile| profile.athlete_id != id)
        })
    {
        bail!("coverage artifact belongs to another athlete");
    }
    let kind = match candidate {
        CandidateCoverage::Complete { .. } => {
            super::complete::verify(
                probe.context("complete coverage has no initial probe")?,
                profile.context("complete coverage has no full acquisition")?,
            )?;
            super::raw_profiles::verify(
                profile.context("complete coverage has no full acquisition")?,
                probe.context("complete coverage has no initial probe")?,
                origin,
                store,
            )?;
            AssessedCoverage::Complete
        }
        CandidateCoverage::Incomplete { .. } => AssessedCoverage::Incomplete,
        CandidateCoverage::NameExcluded { source_name, .. } => {
            super::raw_profiles::verify_probe(
                probe.context("exclusion probe missing")?,
                origin,
                store,
            )?;
            let witness = super::exclusions::verify(
                &row.source,
                source_name,
                probe.context("exclusion probe missing")?,
            )?;
            if !witness
                .documents()
                .iter()
                .all(|digest| reason.evidence().contains(digest))
            {
                bail!("excluded candidate assessment omits identity witness documents");
            }
            AssessedCoverage::NameExcluded
        }
    };
    if reason.coverage() != kind || (kind != AssessedCoverage::Complete && reason.hard_eligible()) {
        bail!("candidate assessment changes acquisition coverage or eligibility");
    }
    Ok(())
}

pub(super) fn verify_selected(report: &RowReport, selected: AthleteId) -> Result<()> {
    if !report.candidates.iter().any(|candidate| matches!(candidate, CandidateCoverage::Complete { athlete_id, .. } if *athlete_id == selected)) {
        bail!("accepted athlete is not a full Complete candidate");
    }
    Ok(())
}
