use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{JoinError, LaneEvidence, LaneSet};
use crate::school_address::Report;

const PROVIDERS: [&str; 3] = ["nces-ccd", "nces-pss", "state-ed"];
pub(super) const ASSOCIATION_PREFIX: &str = "association:";
pub(super) const STATE_ED: &str = "state-ed";

fn admitted_provider(source: &str) -> bool {
    PROVIDERS.contains(&source)
        || source
            .strip_prefix(ASSOCIATION_PREFIX)
            .is_some_and(|slug| !slug.is_empty() && slug.len() <= 64)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Overrides {
    #[serde(default)]
    pub urls: BTreeMap<String, String>,
    #[serde(default)]
    pub dates: BTreeMap<String, String>,
}

impl Overrides {
    pub fn validated(self) -> Result<Self, JoinError> {
        for (source, url) in &self.urls {
            require_provider(source)?;
            match url::Url::parse(url) {
                Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {}
                Ok(_) => {
                    return Err(JoinError::EvidenceValue {
                        kind: "url",
                        pair: format!("{source}={url}"),
                        detail: "url scheme must be http or https".to_string(),
                    });
                }
                Err(error) => {
                    return Err(JoinError::EvidenceValue {
                        kind: "url",
                        pair: format!("{source}={url}"),
                        detail: error.to_string(),
                    });
                }
            }
        }
        for (source, date) in &self.dates {
            require_provider(source)?;
            validate_date(source, date)?;
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

pub fn build_lane_evidence(report: &Report, overrides: &Overrides) -> Result<LaneSet, JoinError> {
    let mut by_provider: BTreeMap<String, Vec<LaneEvidence>> = BTreeMap::new();
    let mut selected: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for lane in &report.lanes {
        if !admitted_provider(&lane.source) {
            continue;
        }
        let selector = capture_selector(&lane.source, &lane.path);
        let lanes = by_provider.entry(lane.source.clone()).or_default();
        if lanes.iter().any(|existing| existing.path == lane.path) {
            continue;
        }
        if overrides.urls.contains_key(&selector) {
            selected.insert(selector.clone());
        }
        if overrides.dates.contains_key(&selector) {
            selected.insert(selector.clone());
        }
        lanes.push(LaneEvidence {
            url: overrides
                .urls
                .get(&selector)
                .or_else(|| overrides.urls.get(&lane.source))
                .cloned(),
            observed_on: overrides
                .dates
                .get(&selector)
                .or_else(|| overrides.dates.get(&lane.source))
                .cloned(),
            path: lane.path.clone(),
            capture_sha256: lane.sha256.clone(),
            generation: report.manifest_digest.clone(),
            captured: lane.captured.clone(),
        });
    }
    for (kind, keys) in [
        ("url", overrides.urls.keys()),
        ("date", overrides.dates.keys()),
    ] {
        for key in keys {
            if is_capture_selector(key) && !selected.contains(key) {
                return Err(JoinError::EvidenceValue {
                    kind,
                    pair: key.clone(),
                    detail: format!(
                        "no capture in this generation matches {key:?}; name a capture as \
                         <provider>@<path>, copying the path from the lane list"
                    ),
                });
            }
        }
    }
    Ok(LaneSet::of(by_provider))
}

fn capture_selector(provider: &str, path: &str) -> String {
    format!("{provider}@{path}")
}

fn is_capture_selector(key: &str) -> bool {
    key.split_once('@')
        .is_some_and(|(provider, path)| !provider.is_empty() && !path.is_empty())
}
