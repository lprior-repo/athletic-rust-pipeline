//! XC distance metadata: canonical `Meters` index and result joins.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::domain::evidence::EvidenceIssue;
use crate::domain::identity::EvidenceDigest;

use super::fields::{bounded_id, display_value, ev, issue, retain_cap_issue};
use super::results::{Distance, ResultContext};
use super::{MAX_DISTANCE_DISPLAY, MAX_ITEMS, MAX_TEXT_BYTES};

pub(super) fn index_distances(
    items: &[Value],
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> BTreeMap<u64, Option<Distance>> {
    retain_cap_issue(
        items.len(),
        "distances_truncated",
        "/distancesXC",
        digest,
        issues,
    );
    items.iter().take(MAX_ITEMS).enumerate().fold(
        BTreeMap::new(),
        |mut distances, (position, value)| {
            let locator = format!("/distancesXC/{position}");
            if let Some((meters, metadata)) = distance_metadata(value, digest, &locator, issues) {
                insert_distance(&mut distances, meters, metadata, digest, &locator, issues);
            }
            distances
        },
    )
}

/// Canonical `Meters` key plus the display label, or `None` after reporting why.
fn distance_metadata(
    value: &Value,
    digest: &EvidenceDigest,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<(u64, Distance)> {
    let Some(object) = value.as_object() else {
        issues.push(issue(
            "invalid_distance_metadata",
            "distance metadata is not an object",
            Some(ev(digest, locator)),
        ));
        return None;
    };
    let meters = distance_meters(object, digest, locator, issues)?;
    let metadata = distance_label(object, digest, locator, issues)?;
    Some((meters, metadata))
}

fn distance_meters(
    object: &Map<String, Value>,
    digest: &EvidenceDigest,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<u64> {
    let Some(meters) = object
        .get("Meters")
        .and_then(Value::as_u64)
        .filter(|value| bounded_id(*value))
    else {
        issues.push(issue(
            "invalid_distance_metadata",
            "distance metadata Meters is not a bounded positive integer",
            Some(ev(digest, locator)),
        ));
        return None;
    };
    Some(meters)
}

fn distance_label(
    object: &Map<String, Value>,
    digest: &EvidenceDigest,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<Distance> {
    let Some(display) = object
        .get("Distance")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= MAX_DISTANCE_DISPLAY)
    else {
        issues.push(issue(
            "invalid_distance_metadata",
            "distance metadata Distance is not a bounded positive number",
            Some(ev(digest, locator)),
        ));
        return None;
    };
    let Some(units) = object
        .get("Units")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= MAX_TEXT_BYTES)
        .map(str::to_owned)
    else {
        issues.push(issue(
            "invalid_distance_metadata",
            "distance metadata Units is not bounded non-empty text",
            Some(ev(digest, locator)),
        ));
        return None;
    };
    let display_value = object
        .get("Distance")
        .map_or_else(|| display.to_string(), display_value);
    Some((format!("{display_value} {units}"), Some(units)))
}

/// First metadata wins; a later differing record makes the key ambiguous.
fn insert_distance(
    distances: &mut BTreeMap<u64, Option<Distance>>,
    meters: u64,
    metadata: Distance,
    digest: &EvidenceDigest,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) {
    match distances.entry(meters) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(Some(metadata));
        }
        std::collections::btree_map::Entry::Occupied(mut entry)
            if entry.get().as_ref() != Some(&metadata) =>
        {
            entry.insert(None);
            issues.push(issue(
                "conflicting_distance_metadata",
                "same canonical Meters key has conflicting metadata",
                Some(ev(digest, locator)),
            ));
        }
        std::collections::btree_map::Entry::Occupied(_) => {}
    }
}
pub(super) fn distance_join(
    id: u64,
    context: &ResultContext<'_>,
    locator: &str,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<Distance> {
    match context.distances.get(&id) {
        Some(Some(distance)) => Some(distance.clone()),
        Some(None) => {
            issues.push(issue(
                "ambiguous_distance_join",
                "XC result Distance joins conflicting distancesXC metadata",
                Some(ev(context.digest, locator)),
            ));
            None
        }
        None => {
            issues.push(issue(
                "missing_distance_join",
                "XC result Distance does not join distancesXC by canonical Meters",
                Some(ev(context.digest, locator)),
            ));
            None
        }
    }
}
