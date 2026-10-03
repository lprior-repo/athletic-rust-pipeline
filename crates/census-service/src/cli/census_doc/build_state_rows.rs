use serde_json::{Map, Value};

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
                .map_or(&empty_map, |value| value);
            let schools = schools_by_state
                .get(state)
                .and_then(|v| v.get("schools"))
                .and_then(|v| v.as_u64())
                .or_else(|| row.get("schools").and_then(|v| v.as_u64()))
                .map_or(Default::default(), core::convert::identity);
            vec![
                state.clone(),
                schools.to_string(),
                row.get("athletes")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
                row.get("class_of_2027")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
                row.get("class_of_2027_boys")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
                row.get("class_of_2027_girls")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
                row.get("class_of_2027_multisource")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
                row.get("class_of_2027_with_coach")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
                row.get("class_of_2027_with_coach_email")
                    .and_then(|v| v.as_u64())
                    .map_or(0, |value| value)
                    .to_string(),
            ]
        })
        .collect()
}
