use std::collections::{BTreeMap, BTreeSet};

use super::input::LegacySourcePairs;
use super::{admitted_provider, lane_evidence, retain_lane, JoinError, LaneEvidence, Report};

pub(super) fn build(
    report: &Report,
    values: &LegacySourcePairs,
) -> Result<BTreeMap<String, Vec<LaneEvidence>>, JoinError> {
    let mut by_provider = BTreeMap::new();
    let mut selected = BTreeSet::new();
    for lane in &report.lanes {
        if !admitted_provider(&lane.source) {
            continue;
        }
        let selector = format!("{}@{}", lane.source, lane.path);
        if values.urls.contains_key(&selector) || values.dates.contains_key(&selector) {
            selected.insert(selector.clone());
        }
        retain_lane(
            by_provider.entry(lane.source.clone()).or_default(),
            lane_evidence(
                report,
                lane,
                value(&values.urls, &lane.source, &selector),
                value(&values.dates, &lane.source, &selector),
            ),
        );
    }
    require_selected("url", &values.urls, &selected)?;
    require_selected("date", &values.dates, &selected)?;
    Ok(by_provider)
}

fn value(values: &BTreeMap<String, String>, provider: &str, selector: &str) -> Option<String> {
    values
        .get(selector)
        .or_else(|| values.get(provider))
        .cloned()
}

fn require_selected(
    kind: &'static str,
    values: &BTreeMap<String, String>,
    selected: &BTreeSet<String>,
) -> Result<(), JoinError> {
    for key in values.keys() {
        if key.contains('@') && !selected.contains(key) {
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
    Ok(())
}
