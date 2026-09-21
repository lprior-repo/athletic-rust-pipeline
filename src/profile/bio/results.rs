//! Sport-result assembly: join indexes, per-result records, and retention caps.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::evidence::{EvidenceIssue, ResultEvidence, Sport};
use crate::domain::identity::{AthleteId, EvidenceDigest};

use super::distances::index_distances;
use super::events;
use super::fields::{bounded_id, ev, issue, optional_text, retain_cap_issue};
use super::record::{parse_result, result_size};
use super::relays::relay_members;
use super::MAX_ITEMS;

pub(super) struct ParsedResults {
    pub(super) results: Vec<ResultEvidence>,
    pub(super) count: u64,
    pub(super) present: bool,
}
/// `Distance` is display event text plus declared units; canonical `Meters` never
/// enters `ResultEvidence.event_name` or `ResultEvidence.units`.
pub(super) type Distance = (String, Option<String>);
pub(super) struct ResultContext<'a> {
    pub(super) athlete: AthleteId,
    pub(super) sport: Sport,
    pub(super) events: &'a events::Index,
    pub(super) distances: &'a BTreeMap<u64, Option<Distance>>,
    pub(super) meets: &'a BTreeMap<u64, Option<String>>,
    pub(super) teams: &'a BTreeSet<u64>,
    pub(super) relays: &'a BTreeMap<u64, BTreeSet<u64>>,
    pub(super) digest: &'a EvidenceDigest,
}

pub(super) fn parse_results(
    root: &Map<String, Value>,
    athlete: AthleteId,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> ParsedResults {
    let key = match sport {
        Sport::TrackField => "resultsTF",
        Sport::CrossCountry => "resultsXC",
    };
    let Some(array) = sport_array(root, key, digest, issues) else {
        return empty_results();
    };
    retain_cap_issue(
        array.len(),
        "results_truncated",
        &format!("/{key}"),
        digest,
        issues,
    );
    sport_shape_issue(root, sport, digest, issues);
    meets_shape_issue(root, digest, issues);
    let events = event_index(root, sport, digest, issues);
    let distances = distance_index(root, sport, digest, issues);
    let meets = meet_index(root);
    let relays = relay_members(root.get("relayTeamMembers"), digest, issues);
    let teams = team_index(root);
    let context = ResultContext {
        athlete,
        sport,
        events: &events,
        distances: &distances,
        meets: &meets,
        teams: &teams,
        relays: &relays,
        digest,
    };
    let results = joined_results(array, key, &context, issues);
    ParsedResults {
        count: u64::try_from(array.len().min(MAX_ITEMS)).map_or(0, |value| value),
        results,
        present: true,
    }
}

fn empty_results() -> ParsedResults {
    ParsedResults {
        results: Vec::new(),
        count: 0,
        present: false,
    }
}

/// Resolve the sport's result array, reporting absence and shape problems in field order.
fn sport_array<'a>(
    root: &'a Map<String, Value>,
    key: &str,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Option<&'a Vec<Value>> {
    let Some(value) = root.get(key) else {
        issues.push(issue(
            "missing_sport_results",
            "sport result array is absent",
            Some(ev(digest, format!("/{key}"))),
        ));
        return None;
    };
    if value.is_null() {
        return None;
    }
    let Some(array) = value.as_array() else {
        issues.push(issue(
            "unknown_results_shape",
            "sport result field is neither null nor an array",
            Some(ev(digest, format!("/{key}"))),
        ));
        return None;
    };
    Some(array)
}

/// The sport's own event or distance collection is optional but, when present, must be an array.
fn sport_shape_issue(
    root: &Map<String, Value>,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) {
    if sport == Sport::TrackField
        && root
            .get("eventsTF")
            .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_events_shape",
            "eventsTF is neither null nor an array",
            Some(ev(digest, "/eventsTF")),
        ));
    }
    if sport == Sport::CrossCountry
        && root
            .get("distancesXC")
            .is_some_and(|value| !value.is_array() && !value.is_null())
    {
        issues.push(issue(
            "unknown_distances_shape",
            "distancesXC is neither null nor an array",
            Some(ev(digest, "/distancesXC")),
        ));
    }
}

/// Meets are optional but, when present, must be an object.
fn meets_shape_issue(
    root: &Map<String, Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) {
    if root
        .get("meets")
        .is_some_and(|value| !value.is_object() && !value.is_null())
    {
        issues.push(issue(
            "unknown_meets_shape",
            "meets is neither null nor an object",
            Some(ev(digest, "/meets")),
        ));
    }
}

fn event_index(
    root: &Map<String, Value>,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> events::Index {
    if sport == Sport::TrackField {
        root.get("eventsTF")
            .and_then(Value::as_array)
            .map_or_else(BTreeMap::new, |items| events::index(items, digest, issues))
    } else {
        BTreeMap::new()
    }
}

fn distance_index(
    root: &Map<String, Value>,
    sport: Sport,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> BTreeMap<u64, Option<Distance>> {
    if sport == Sport::CrossCountry {
        root.get("distancesXC")
            .and_then(Value::as_array)
            .map_or_else(BTreeMap::new, |items| {
                index_distances(items, digest, issues)
            })
    } else {
        BTreeMap::new()
    }
}

fn meet_index(root: &Map<String, Value>) -> BTreeMap<u64, Option<String>> {
    root.get("meets")
        .and_then(Value::as_object)
        .map_or_else(BTreeMap::new, |map| {
            map.iter()
                .take(MAX_ITEMS)
                .filter_map(|(key, value)| {
                    key.parse::<u64>().ok().map(|id| {
                        (
                            id,
                            value
                                .as_object()
                                .and_then(|obj| optional_text(obj.get("MeetName"))),
                        )
                    })
                })
                .collect::<BTreeMap<_, _>>()
        })
}

fn team_index(root: &Map<String, Value>) -> BTreeSet<u64> {
    root.get("allTeams")
        .and_then(Value::as_object)
        .map_or_else(BTreeSet::new, |map| {
            map.keys()
                .take(MAX_ITEMS)
                .filter_map(|key| key.parse::<u64>().ok().filter(|value| bounded_id(*value)))
                .collect::<BTreeSet<_>>()
        })
}

fn joined_results(
    array: &[Value],
    key: &str,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> Vec<ResultEvidence> {
    array
        .iter()
        .take(MAX_ITEMS)
        .enumerate()
        .scan(0_usize, |bytes, (index, item)| {
            let record = parse_result(item, index, context, issues);
            within_result_budget(&record, bytes, key, index, context, issues).then_some(record)
        })
        .flatten()
        .collect::<Vec<_>>()
}

/// Running 8 MiB cap over materialized records; an over-budget record is dropped and reported.
fn within_result_budget(
    record: &Option<ResultEvidence>,
    bytes: &mut usize,
    key: &str,
    index: usize,
    context: &ResultContext<'_>,
    issues: &mut Vec<EvidenceIssue>,
) -> bool {
    let size = record.as_ref().map_or(Some(0), result_size);
    let total = size
        .and_then(|size| bytes.checked_add(size))
        .filter(|total| *total <= 8 * 1024 * 1024);
    match total {
        Some(total) => {
            *bytes = total;
            true
        }
        None => {
            issues.push(issue(
                "joined_results_truncated",
                "materialized result evidence exceeds 8 MiB; raw document retained",
                Some(ev(context.digest, format!("/{key}/{index}"))),
            ));
            false
        }
    }
}
