//! Relay membership index and result attribution.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::evidence::{EvidenceIssue, ResultAttribution};
use crate::domain::identity::{AthleteId, EvidenceDigest};

use super::fields::{bounded_id, ev, issue};
use super::MAX_ITEMS;

pub(super) fn relay_members(
    value: Option<&Value>,
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> BTreeMap<u64, BTreeSet<u64>> {
    let Some(value) = value.filter(|item| !item.is_null()) else {
        return BTreeMap::new();
    };
    let Some(items) = value.as_array() else {
        issues.push(issue(
            "unknown_relay_members_shape",
            "relayTeamMembers is not an array",
            Some(ev(digest, "/relayTeamMembers")),
        ));
        return BTreeMap::new();
    };
    if items.len() > MAX_ITEMS {
        issues.push(issue(
            "relay_members_truncated",
            "relay membership exceeds parser bound",
            Some(ev(digest, "/relayTeamMembers")),
        ));
    }
    items
        .iter()
        .take(MAX_ITEMS)
        .enumerate()
        .fold(BTreeMap::new(), |mut members, (index, item)| {
            let team = item
                .get("TeamID")
                .and_then(Value::as_u64)
                .filter(|id| bounded_id(*id));
            let athlete = item
                .get("AthleteID")
                .and_then(Value::as_u64)
                .filter(|id| bounded_id(*id));
            match (team, athlete) {
                (Some(team), Some(athlete)) => {
                    members.entry(team).or_default().insert(athlete);
                }
                _ => issues.push(issue(
                    "invalid_relay_member",
                    "relay membership requires positive TeamID and AthleteID",
                    Some(ev(digest, format!("/relayTeamMembers/{index}"))),
                )),
            }
            members
        })
}
pub(super) fn attribution(
    athlete: AthleteId,
    reported: Option<u64>,
    personal: bool,
    relays: &BTreeMap<u64, BTreeSet<u64>>,
) -> ResultAttribution {
    match (reported, personal) {
        (Some(id), true) if id == athlete.get() => ResultAttribution::Individual,
        (Some(id), false)
            if relays
                .get(&id)
                .is_some_and(|members| members.contains(&athlete.get())) =>
        {
            ResultAttribution::VerifiedRelayMember {
                relay_athlete_id: id,
            }
        }
        (id, _) => ResultAttribution::Unresolved {
            reported_athlete_id: id,
        },
    }
}
