use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::{JoinError, LaneEvidence, LaneSet};
use crate::school_address::{LaneReport, Report};

pub(super) mod input;

use input::EvidenceInput;
pub use input::{Acquisition, Overrides};

const PROVIDERS: [&str; 3] = ["nces-ccd", "nces-pss", "state-ed"];
pub(super) const ASSOCIATION_PREFIX: &str = "association:";
pub(super) const STATE_ED: &str = "state-ed";

type LaneMatches = (BTreeMap<String, Vec<LaneEvidence>>, BTreeSet<u32>);

fn admitted_provider(source: &str) -> bool {
    PROVIDERS.contains(&source)
        || source
            .strip_prefix(ASSOCIATION_PREFIX)
            .is_some_and(|slug| !slug.is_empty() && slug.len() <= 64)
}

pub fn build_lane_evidence(report: &Report, overrides: &Overrides) -> Result<LaneSet, JoinError> {
    let EvidenceInput::Acquisitions(values) = &overrides.input;
    let by_provider = acquisition_evidence(report, values)?;
    Ok(LaneSet::of(by_provider))
}

fn acquisition_evidence(
    report: &Report,
    acquisitions: &[Acquisition],
) -> Result<BTreeMap<String, Vec<LaneEvidence>>, JoinError> {
    let (by_provider, matched_entries) = match_acquisitions_to_lanes(report, acquisitions);
    for acquisition in acquisitions {
        if !matched_entries.contains(&acquisition.entry) {
            return Err(JoinError::EvidenceValue {
                kind: "url",
                pair: format!("{}@{}", acquisition.provider, acquisition.path),
                detail: "acquisition matched no generation lane".to_string(),
            });
        }
    }
    Ok(by_provider)
}

fn match_acquisitions_to_lanes(report: &Report, acquisitions: &[Acquisition]) -> LaneMatches {
    let mut by_provider: BTreeMap<String, Vec<LaneEvidence>> = BTreeMap::new();
    let mut matched_entries = BTreeSet::new();
    for lane in &report.lanes {
        if !admitted_provider(&lane.source) {
            continue;
        }
        let mut matched = false;
        for acquisition in acquisitions {
            if !matches_lane(acquisition, lane) {
                continue;
            }
            matched = true;
            matched_entries.insert(acquisition.entry);
            retain_lane(
                by_provider.entry(lane.source.clone()).or_default(),
                lane_evidence(
                    report,
                    lane,
                    Some(acquisition.url.clone()),
                    acquisition.retrieved_at.clone(),
                ),
            );
        }
        if !matched {
            retain_lane(
                by_provider.entry(lane.source.clone()).or_default(),
                lane_evidence(report, lane, None, None),
            );
        }
    }
    (by_provider, matched_entries)
}

fn matches_lane(acquisition: &Acquisition, lane: &LaneReport) -> bool {
    if acquisition.provider != lane.source {
        return false;
    }
    if acquisition.path.is_empty() {
        return true;
    }
    let prefix = format!("{}@", lane.source);
    let acquisition_path = acquisition
        .path
        .strip_prefix(&prefix)
        .map_or(acquisition.path.as_str(), core::convert::identity);
    canonical_or_literal(acquisition_path) == canonical_or_literal(&lane.path)
}

fn canonical_or_literal(value: &str) -> PathBuf {
    match std::fs::canonicalize(Path::new(value)) {
        Ok(path) => path,
        Err(_) => PathBuf::from(value),
    }
}

fn lane_evidence(
    report: &Report,
    lane: &LaneReport,
    url: Option<String>,
    observed_on: Option<String>,
) -> LaneEvidence {
    LaneEvidence {
        url,
        observed_on,
        path: lane.path.clone(),
        capture_sha256: lane.sha256.clone(),
        generation: report.manifest_digest.clone(),
        captured: lane.captured.clone(),
    }
}

fn retain_lane(lanes: &mut Vec<LaneEvidence>, mut evidence: LaneEvidence) {
    evidence.captured.sort();
    if !lanes
        .iter()
        .any(|existing| identical_lane(existing, &evidence))
    {
        lanes.push(evidence);
    }
}

fn identical_lane(left: &LaneEvidence, right: &LaneEvidence) -> bool {
    left.url == right.url
        && left.observed_on == right.observed_on
        && left.path == right.path
        && left.capture_sha256 == right.capture_sha256
        && left.generation == right.generation
        && left.captured == right.captured
}
