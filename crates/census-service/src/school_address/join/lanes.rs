use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{JoinError, LaneEvidence, LaneSet};
use crate::school_address::Report;

const PROVIDERS: [&str; 3] = ["nces-ccd", "nces-pss", "state-ed"];
pub(super) const ASSOCIATION_PREFIX: &str = "association:";
pub(super) const STATE_ED: &str = "state-ed";

type LaneMatches = (BTreeMap<String, Vec<LaneEvidence>>, std::collections::BTreeSet<u32>);

fn admitted_provider(source: &str) -> bool {
    PROVIDERS.contains(&source)
        || source
            .strip_prefix(ASSOCIATION_PREFIX)
            .is_some_and(|slug| !slug.is_empty() && slug.len() <= 64)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Acquisition {
    pub entry: u32,
    pub provider: String,
    pub path: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieved_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub acquisitions: Vec<Acquisition>,
}

impl Overrides {
    pub fn validated(self) -> Result<Self, JoinError> {
        for acq in &self.acquisitions {
            require_provider(&acq.provider)?;
            match url::Url::parse(&acq.url) {
                Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {}
                Ok(_) => {
                    return Err(JoinError::EvidenceValue {
                        kind: "url",
                        pair: format!("{}={}", acq.provider, acq.url),
                        detail: "url scheme must be http or https".to_string(),
                    });
                }
                Err(error) => {
                    return Err(JoinError::EvidenceValue {
                        kind: "url",
                        pair: format!("{}={}", acq.provider, acq.url),
                        detail: error.to_string(),
                    });
                }
            }
            if let Some(date) = &acq.retrieved_at {
                validate_date(&acq.provider, date)?;
            }
        }
        Ok(self)
    }
}

pub fn parse_source_pairs(values: &[String]) -> Result<BTreeMap<String, String>, JoinError> {
    let mut pairs = BTreeMap::new();
    for value in values {
        let Some((source, target)) = value.split_once('=') else {
            return Err(JoinError::EvidencePair {
                pair: value.clone(),
            });
        };
        if source.is_empty() || target.is_empty() {
            return Err(JoinError::EvidencePair {
                pair: value.clone(),
            });
        }
        pairs.insert(source.to_string(), target.to_string());
    }
    Ok(pairs)
}

fn require_provider(source: &str) -> Result<(), JoinError> {
    let provider = source
        .split_once('@')
        .map_or(source, |(provider, _)| provider);
    if admitted_provider(provider) {
        return Ok(());
    }
    Err(JoinError::EvidenceSource {
        provider: source.to_string(),
    })
}

fn validate_date(source: &str, value: &str) -> Result<(), JoinError> {
    let trimmed = value.trim();
    let valid = if trimmed.len() == 10 {
        chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").is_ok()
    } else {
        chrono::DateTime::parse_from_rfc3339(trimmed).is_ok()
    };
    if valid {
        return Ok(());
    }
    Err(JoinError::EvidenceValue {
        kind: "date",
        pair: format!("{source}={value}"),
        detail: "expected a YYYY-MM-DD date or RFC3339 timestamp".to_string(),
    })
}

fn match_acquisitions_to_lanes(
    report: &Report,
    overrides: &Overrides,
) -> Result<LaneMatches, JoinError> {
    let mut by_provider: BTreeMap<String, Vec<LaneEvidence>> = BTreeMap::new();
    let mut matched_entries = std::collections::BTreeSet::new();

    for lane in &report.lanes {
        if !admitted_provider(&lane.source) {
            continue;
        }

        let matching_acqs: Vec<&Acquisition> = overrides
            .acquisitions
            .iter()
            .filter(|acq| {
                if acq.provider != lane.source {
                    return false;
                }
                if acq.path.is_empty() {
                    return true;
                }
                let acq_path = acq.path
                    .strip_prefix(&format!("{}@", lane.source))
                    .unwrap_or(&acq.path);
                let acq_canonicalized = match std::fs::canonicalize(acq_path) {
                    Ok(p) => p,
                    Err(_) => std::path::PathBuf::from(acq_path),
                };
                let lane_canonicalized = match std::fs::canonicalize(&lane.path) {
                    Ok(p) => p,
                    Err(_) => std::path::PathBuf::from(&lane.path),
                };
                acq_canonicalized == lane_canonicalized
            })
            .collect();

        for acq in &matching_acqs {
            matched_entries.insert(acq.entry);
            by_provider
                .entry(lane.source.clone())
                .or_default()
                .push(LaneEvidence {
                    url: Some(acq.url.clone()),
                    observed_on: acq.retrieved_at.clone(),
                    path: lane.path.clone(),
                    capture_sha256: lane.sha256.clone(),
                    generation: report.manifest_digest.clone(),
                    captured: lane.captured.clone(),
                });
        }

        if matching_acqs.is_empty() {
            by_provider
                .entry(lane.source.clone())
                .or_default()
                .push(LaneEvidence {
                    url: None,
                    observed_on: None,
                    path: lane.path.clone(),
                    capture_sha256: lane.sha256.clone(),
                    generation: report.manifest_digest.clone(),
                    captured: lane.captured.clone(),
                });
        }
    }

    Ok((by_provider, matched_entries))
}

pub fn build_lane_evidence(report: &Report, overrides: &Overrides) -> Result<LaneSet, JoinError> {
    let (by_provider, matched_entries) = match_acquisitions_to_lanes(report, overrides)?;

    for acq in &overrides.acquisitions {
        if !matched_entries.contains(&acq.entry) {
            return Err(JoinError::EvidenceValue {
                kind: "url",
                pair: format!("{}@{}", acq.provider, acq.path),
                detail: "acquisition matched no generation lane".to_string(),
            });
        }
    }

    Ok(LaneSet::of(by_provider))
}
