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
    let Some(value) = root.get(key) else {
        issues.push(issue(
            "missing_sport_results",
            "sport result array is absent",
            Some(ev(digest, format!("/{key}"))),
        ));
        return ParsedResults {
            results: Vec::new(),
            count: 0,
            present: false,
        };
    };
    if value.is_null() {
        return ParsedResults {
            results: Vec::new(),
            count: 0,
            present: false,
        };
    }
    let Some(array) = value.as_array() else {
        issues.push(issue(
            "unknown_results_shape",
            "sport result field is neither null nor an array",
            Some(ev(digest, format!("/{key}"))),
        ));
        return ParsedResults {
            results: Vec::new(),
            count: 0,
            present: false,
        };
    };
    retain_cap_issue(
        array.len(),
        "results_truncated",
        &format!("/{key}"),
        digest,
        issues,
    );
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
    let events = if sport == Sport::TrackField {
        root.get("eventsTF")
            .and_then(Value::as_array)
            .map_or_else(BTreeMap::new, |items| events::index(items, digest, issues))
    } else {
        BTreeMap::new()
    };
    let distances = if sport == Sport::CrossCountry {
        root.get("distancesXC")
            .and_then(Value::as_array)
            .map_or_else(BTreeMap::new, |items| {
                index_distances(items, digest, issues)
            })
    } else {
        BTreeMap::new()
    };
    let meets = root
        .get("meets")
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
        });
    let relays = relay_members(root.get("relayTeamMembers"), digest, issues);
    let teams = root
        .get("allTeams")
        .and_then(Value::as_object)
        .map_or_else(BTreeSet::new, |map| {
            map.keys()
                .take(MAX_ITEMS)
                .filter_map(|key| key.parse::<u64>().ok().filter(|value| bounded_id(*value)))
                .collect::<BTreeSet<_>>()
        });
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
    let results = array
        .iter()
        .take(MAX_ITEMS)
        .enumerate()
        .scan(0_usize, |bytes, (index, item)| {
            let record = parse_result(item, index, &context, issues);
            let size = record.as_ref().map_or(Some(0), result_size);
            let total = size
                .and_then(|size| bytes.checked_add(size))
                .filter(|total| *total <= 8 * 1024 * 1024);
            match total {
                Some(total) => {
                    *bytes = total;
                    Some(record)
                }
                None => {
                    issues.push(issue(
                        "joined_results_truncated",
                        "materialized result evidence exceeds 8 MiB; raw document retained",
                        Some(ev(digest, format!("/{key}/{index}"))),
                    ));
                    None
                }
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    ParsedResults {
        count: u64::try_from(array.len().min(MAX_ITEMS)).map_or(0, |value| value),
        results,
        present: true,
    }
}
