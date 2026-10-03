use anyhow::Result;
use census_domain::model::serialized_digest;
use census_service::restate_services::TeamsSourceRequest;
use serde_json::{json, Value};

use super::super::super::ledger::{Snapshot, PHASE};

pub(super) fn boundary(
    snapshot: &Snapshot,
    key: &str,
    input: &TeamsSourceRequest,
    progress: &Value,
) -> Result<bool> {
    let prefix = format!("{key}/");
    let exact = exact_keys(snapshot, &prefix);
    let digest = serialized_digest(&(
        input.jurisdiction.jurisdiction,
        input.jurisdiction.season,
        input.jurisdiction.revision,
        &input.source,
    ))?;
    let identity = snapshot.entries.get(&format!("{prefix}identity"));
    let metadata = identity.is_some_and(|value| {
        value.get("request_digest").and_then(Value::as_str) == Some(digest.as_str())
            && value.get("observed_on").and_then(Value::as_str) == Some(input.observed_on.as_str())
    });
    let reservations = (1..=3).all(|attempt| {
        snapshot
            .entries
            .get(&format!("{prefix}attempt/{attempt}/reserved"))
            .is_some_and(|value| {
                value.get("attempt") == Some(&json!(attempt))
                    && value.get("observed_on").and_then(Value::as_str)
                        == Some(input.observed_on.as_str())
            })
    });
    let reports = (1..=2)
        .map(|attempt| {
            let recorded = snapshot
                .entries
                .get(&format!("{prefix}attempt/{attempt}/outcome"))?;
            if recorded.get("status").and_then(Value::as_str) != Some("transient")
                || recorded.get("message").and_then(Value::as_str).is_none()
            {
                return None;
            }
            Some(json!({"status":"transient", "attempt":attempt,
            "outcome":recorded.get("progress"), "message":recorded.get("message")}))
        })
        .collect::<Option<Vec<_>>>();
    let same = reports.is_some_and(|mut reports| {
        reports.push(json!({"status":"unknown", "attempt":3}));
        progress == &json!(reports)
    });
    Ok(snapshot.phase == PHASE && exact && metadata && reservations && same)
}

fn exact_keys(snapshot: &Snapshot, prefix: &str) -> bool {
    let suffixes = [
        "identity",
        "attempt/1/reserved",
        "attempt/1/outcome",
        "attempt/2/reserved",
        "attempt/2/outcome",
        "attempt/3/reserved",
    ];
    snapshot
        .entries
        .keys()
        .filter(|name| name.starts_with(prefix))
        .count()
        == suffixes.len()
        && suffixes
            .iter()
            .all(|suffix| snapshot.entries.contains_key(&format!("{prefix}{suffix}")))
}

pub(super) fn unchanged(before: &Snapshot, after: &Snapshot, key: &str) -> bool {
    let prefix = format!("{key}/");
    before.phase == after.phase
        && before.sequence <= after.sequence
        && before
            .entries
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
            .all(|(name, value)| after.entries.get(name) == Some(value))
        && !after
            .entries
            .contains_key(&format!("{prefix}attempt/3/outcome"))
}
