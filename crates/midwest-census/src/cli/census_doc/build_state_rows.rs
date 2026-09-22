//! Build the "By state" table rows.

use serde_json::{Map, Value};

/// Build state rows from the parsed report.
///
/// `empty_map` is a zero-length map used as a fallback when a state key
/// is absent from `by_state`.  It is created once by the caller.
pub(super) fn build(
    by_state: &Map<String, Value>,
    schools_by_state: &Map<String, Value>,
) -> Vec<Vec<String>> {
    let empty_map = Map::new();
    let states: Vec<String> = {
        let mut s: Vec<String> = by_state.keys().cloned().collect();
        s.sort();
        s
    };

    states
        .iter()
        .map(|state| {
            let row = by_state
                .get(state)
                .and_then(|v| v.as_object())
                .unwrap_or(&empty_map);
            let schools = schools_by_state
                .get(state)
                .and_then(|v| v.get("schools"))
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| row.get("schools").and_then(|v| v.as_u64()).unwrap_or(0));
            vec![
                state.clone(),
                schools.to_string(),
                row.get("athletes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
                row.get("class_of_2027")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
                row.get("class_of_2027_boys")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
                row.get("class_of_2027_girls")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
                row.get("class_of_2027_multisource")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
                row.get("class_of_2027_with_coach")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
                row.get("class_of_2027_with_coach_email")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .to_string(),
            ]
        })
        .collect()
}
