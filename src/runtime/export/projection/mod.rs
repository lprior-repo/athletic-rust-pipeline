//! Row projection: what one accepted row contributes to the workbook.
//!
//! Every item below was previously declared in `export/projection.rs`; the `pub(crate)` surface
//! consumed by `export::stream` and `export::details` is unchanged, and the sibling modules hold
//! the field mapping, the roster annotations and the PR-summary emission path.

use super::index::load_json;
// The sibling modules reach these names as `super::<Type>`, the same path text they used when the
// whole module was one file.
use super::{AcceptanceMethod, RowReport, RowResolution};
use crate::store::ArtifactStore;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

mod field_map;
mod roster;
mod summary;

use self::field_map::{acceptance, assessment_summary, fields};
use self::roster::annotations_for;
#[cfg(test)]
use self::roster::AcceptedAnnotations;
use self::summary::build_pr_summary;
#[cfg(test)]
use self::summary::MAX_EXCEL_CELL_UTF16_UNITS;

#[derive(Debug)]
pub(crate) struct Projection {
    pub(crate) report: super::RowReport,
    pub(crate) discovery: Option<Value>,
    pub(crate) identity_artifacts: Vec<Value>,
    pub(crate) assessment: Option<Value>,
    pub(crate) profile_artifacts: Vec<Value>,
    pub(crate) fields: BTreeMap<String, String>,
}

pub(crate) fn project_report(
    store: &ArtifactStore,
    digest: &crate::domain::identity::EvidenceDigest,
) -> Result<Projection> {
    let report: super::RowReport = load_json(store, digest)?;
    let artifacts = load_artifacts(store, &report)?;
    let profiles = load_acquisitions(store, &report)?;
    let accepted = acceptance(&report.resolution);
    let selected_profiles = accepted.map_or_else(Vec::new, |(athlete_id, _)| {
        profiles
            .iter()
            .filter(|acquisition| {
                acquisition.complete
                    && acquisition.athlete_id == athlete_id
                    && acquisition.profile.is_some()
            })
            .filter_map(|acquisition| acquisition.profile.clone())
            .collect::<Vec<_>>()
    });
    let annotations = annotations_for(&selected_profiles, accepted.map(|(id, _)| id));
    let (candidate_count, strength) = artifacts
        .assessment
        .as_ref()
        .map_or((String::new(), String::new()), assessment_summary);
    let pr_summary = match accepted {
        Some((athlete_id, _)) => build_pr_summary(
            &selected_profiles,
            athlete_id,
            report.job.source.as_str(),
            digest,
        )?,
        None => String::new(),
    };
    let fields = fields(
        &report,
        digest,
        accepted,
        annotations,
        candidate_count,
        strength,
        pr_summary,
    );
    Ok(Projection {
        report,
        discovery: artifacts.discovery,
        identity_artifacts: artifacts.identity_artifacts,
        assessment: artifacts.assessment,
        profile_artifacts: artifacts.profile_artifacts,
        fields,
    })
}

/// Every optional artifact one report row points at, decoded once each.
struct LoadedArtifacts {
    discovery: Option<Value>,
    identity_artifacts: Vec<Value>,
    assessment: Option<Value>,
    profile_artifacts: Vec<Value>,
}

fn load_artifacts(store: &ArtifactStore, report: &RowReport) -> Result<LoadedArtifacts> {
    let discovery = report
        .discovery
        .as_ref()
        .map(|value| load_json(store, value))
        .transpose()?;
    let assessment = report
        .assessment
        .as_ref()
        .map(|value| load_json(store, value))
        .transpose()?;
    let identity_artifacts = report
        .candidates
        .iter()
        .filter_map(|candidate| candidate.probe())
        .scan(BTreeSet::new(), |seen, value| {
            seen.insert(value.as_str().to_owned()).then_some(value)
        })
        .map(|value| load_json(store, value))
        .collect::<Result<Vec<Value>>>()?;
    let profile_artifacts = report
        .candidates
        .iter()
        .filter_map(|candidate| candidate.profile())
        .map(|value| load_profile(store, value).map(|(raw, _)| raw))
        .collect::<Result<Vec<_>>>()?;
    Ok(LoadedArtifacts {
        discovery,
        identity_artifacts,
        assessment,
        profile_artifacts,
    })
}

fn load_acquisitions(
    store: &ArtifactStore,
    report: &RowReport,
) -> Result<Vec<super::ProfileAcquisition>> {
    let loaded = report
        .candidates
        .iter()
        .filter_map(|candidate| match candidate {
            super::CandidateCoverage::Complete { profile, .. } => {
                Some(load_profile(store, profile))
            }
            _ => None,
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(loaded
        .into_iter()
        .map(|(_, acquisition)| acquisition)
        .collect())
}

fn load_profile(
    store: &ArtifactStore,
    digest: &crate::domain::identity::EvidenceDigest,
) -> Result<(Value, super::ProfileAcquisition)> {
    let value: Value = load_json(store, digest)?;
    let acquisition =
        serde_json::from_value(value.clone()).context("decoding profile acquisition")?;
    Ok((value, acquisition))
}

pub(crate) fn raw_performance_values(profile: &Value) -> Vec<Value> {
    profile
        .get("profile")
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |results| results.clone())
}

#[cfg(test)]
mod tests;
