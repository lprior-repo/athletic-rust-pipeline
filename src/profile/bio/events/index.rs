//! Assembly of the `eventsTF` index and the result-side join into it.

use super::super::{ev, issue, retain_cap_issue, MAX_ITEMS};
use super::schema::{parse, variant};
use super::{Event, Index};
use crate::domain::{evidence::EvidenceIssue, identity::EvidenceDigest};
use serde_json::{Map, Value};

pub(in crate::profile::bio) fn index(
    items: &[Value],
    digest: &EvidenceDigest,
    issues: &mut Vec<EvidenceIssue>,
) -> Index {
    retain_cap_issue(items.len(), "events_truncated", "/eventsTF", digest, issues);
    items
        .iter()
        .take(MAX_ITEMS)
        .enumerate()
        .fold(Index::new(), |mut events, (position, value)| {
            let locator = format!("/eventsTF/{position}");
            let Some(object) = value.as_object() else {
                issues.push(issue(
                    "invalid_event_metadata",
                    "event metadata is not an object",
                    Some(ev(digest, locator)),
                ));
                return events;
            };
            let parsed = parse(object);
            let Some((key, event)) = parsed else {
                issues.push(issue(
                    "invalid_event_metadata",
                    "event metadata has invalid identity or exceeds text bounds",
                    Some(ev(digest, locator)),
                ));
                return events;
            };
            match events.entry(key) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Some(event));
                }
                std::collections::btree_map::Entry::Occupied(mut entry)
                    if entry.get().as_ref() != Some(&event) =>
                {
                    entry.insert(None);
                    issues.push(issue(
                        "conflicting_event_metadata",
                        "same event and variant have conflicting metadata",
                        Some(ev(digest, locator)),
                    ));
                }
                std::collections::btree_map::Entry::Occupied(_) => {}
            }
            events
        })
}

pub(in crate::profile::bio) fn lookup<'a>(
    events: &'a Index,
    id: u64,
    result: &Map<String, Value>,
) -> Option<&'a Event> {
    let variant = variant(result.get("EventTypeID"))?;
    events.get(&(id, variant)).and_then(Option::as_ref)
}
