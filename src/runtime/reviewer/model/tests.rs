use super::*;
use crate::runtime::protocol::ReviewInput;
use serde_json::json;

fn review_input() -> ReviewInput {
    serde_json::from_value(json!({
        "source": {"source_key":"Sheet:2","sheet":"Sheet","excel_row":2,"fields":{"Person First":"Ada"}},
        "candidates": [{
            "athlete_id": 7,
            "name": "Ada Runner",
            "teams": [{"team_id": 9,"name":{"value":"Central","evidence":{"document":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","locator":"team/name"}},"location":null,"seasons":[],"level":null}],
            "graduation_years": [], "sports": [], "issues": [],
            "eligibility_reasons": ["matching sport"], "documents": []
        }]
    })).expect("synthetic review input")
}

#[test]
fn stable_selected_case_requires_supplied_evidence() {
    let response = json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"decision\":\"select\",\"athlete_id\":7,\"reason\":\"same team\",\"evidence\":[{\"document\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"locator\":\"team/name\"}]}","refusal":null}}]});
    let verdict = parse_response(response.to_string().as_bytes(), &review_input())
        .expect("selected verdict");
    assert!(
        matches!(verdict, AssistantVerdict::Select { athlete_id, .. } if athlete_id.get() == 7)
    );
}

#[test]
fn invented_id_and_reference_are_rejected() {
    let response = json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"decision\":\"select\",\"athlete_id\":8,\"reason\":\"guess\",\"evidence\":[]}"}}]});
    assert!(parse_response(response.to_string().as_bytes(), &review_input()).is_err());
}

#[test]
fn known_candidate_cannot_cite_unsupported_reference() {
    let response = json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"decision\":\"select\",\"athlete_id\":7,\"reason\":\"same team\",\"evidence\":[{\"document\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"locator\":\"team/name\"}]}"}}]});
    assert!(parse_response(response.to_string().as_bytes(), &review_input()).is_err());
}

#[test]
fn refusal_truncation_and_ambiguous_shape_are_rejected() {
    let refusal =
        json!({"choices":[{"finish_reason":"stop","message":{"content":"{}","refusal":"no"}}]});
    let truncated = json!({"choices":[{"finish_reason":"length","message":{"content":"{}"}}]});
    let ambiguous = json!({"choices":[{"finish_reason":"stop","message":{"content":"{\"decision\":\"review\"}"}}]});
    for response in [refusal, truncated, ambiguous] {
        assert!(parse_response(response.to_string().as_bytes(), &review_input()).is_err());
    }
}
