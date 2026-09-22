//! Build meets-by-state and provider table rows from the parsed report.

use serde_json::{Map, Value};

/// Returns `(meets_by_state_rows, top_provider_rows, provider_count)`.
pub(super) fn build(meets: &Map<String, Value>) -> (Vec<Vec<String>>, Vec<Vec<String>>, usize) {
    // ── by_state ───────────────────────────────────────────────────
    let meets_by_state = meets
        .get("by_state")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let meets_by_state_rows: Vec<Vec<String>> = {
        let mut entries: Vec<(String, u64)> = meets_by_state
            .iter()
            .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n)))
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
            .iter()
            .map(|(k, n)| vec![k.clone(), n.to_string()])
            .collect()
    };

    // ── by_provider ────────────────────────────────────────────────
    let providers = meets
        .get("by_provider")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let provider_count = providers.len();
    let mut provider_entries: Vec<(String, u64)> = providers
        .iter()
        .filter_map(|(k, v)| v.as_u64().map(|n| (k.clone(), n)))
        .collect();
    provider_entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top_provider_rows: Vec<Vec<String>> = provider_entries
        .iter()
        .take(10)
        .map(|(k, n)| vec![k.clone(), n.to_string()])
        .collect();

    (meets_by_state_rows, top_provider_rows, provider_count)
}
