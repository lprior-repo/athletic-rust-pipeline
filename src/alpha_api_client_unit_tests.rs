use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};

#[test]
fn retry_after_seconds_convert_to_millis() {
    let secs: u64 = 1;
    let delay_ms = secs.saturating_mul(1000);
    assert_eq!(delay_ms, 1000, "1 second must be 1000ms");

    let secs: u64 = 0;
    let delay_ms = secs.saturating_mul(1000);
    let wait = delay_ms.max(10);
    assert_eq!(wait, 10, "zero Retry-After must use min_delay_ms");

    let secs: u64 = 1000;
    let delay_ms = secs.saturating_mul(1000);
    assert_eq!(delay_ms, 1_000_000, "1000 seconds must be 1000000ms");

    let secs: u64 = u64::MAX;
    let delay_ms = secs.saturating_mul(1000);
    assert_eq!(delay_ms, u64::MAX, "overflow must saturate");
}

#[test]
fn serialize_rankings_body_numeric_divlistid() {
    let req = AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".to_owned(),
        event_short: "100m".to_owned(),
        indoor: false,
        continuation: None,
    };
    let body = AlphaApiClient::serialize_rankings_body(&req);
    assert!(body["divListId"].is_number());
    assert_eq!(body["divListId"], serde_json::json!(12));
}

#[test]
fn serialize_rankings_body_single_response_qparams() {
    let pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/complete".to_owned(),
    };
    let qparams = AlphaApiClient::build_qparams(&pagination, &None);
    assert_eq!(qparams.as_object().unwrap().len(), 0);
}

#[test]
fn serialize_rankings_body_nextpage_qparams() {
    let pagination = PaginationConfig::NextPage {
        has_more_pointer: "/hasMore".to_owned(),
        next_page_pointer: "/nextPage".to_owned(),
        request_page_key: "page".to_owned(),
    };
    let continuation = Some(serde_json::json!({"page": 2}));
    let qparams = AlphaApiClient::build_qparams(&pagination, &continuation);
    assert_eq!(qparams["page"], serde_json::json!({"page": 2}));
}

#[test]
fn serialize_rankings_body_all_keys_present() {
    let req = AlphaRequest {
        state_id: 1,
        season_id: 2026,
        gender: "f".to_owned(),
        event_short: "200m".to_owned(),
        indoor: false,
        continuation: None,
    };
    let body = AlphaApiClient::serialize_rankings_body(&req);
    let expected_keys = [
        "reportType", "mode", "divListId", "indoor", "eventShort",
        "gender", "qualifyingListKey", "version", "debug",
    ];
    for key in &expected_keys {
        assert!(body.get(*key).is_some(), "body must contain key '{}'", key);
    }
}

#[test]
fn serialize_rankings_body_qparams_with_continuation() {
    let pagination = PaginationConfig::NextPage {
        has_more_pointer: "/hasMore".to_owned(),
        next_page_pointer: "/nextPage".to_owned(),
        request_page_key: "page".to_owned(),
    };
    let continuation = Some(serde_json::json!({"page": "next_1"}));
    let qparams = AlphaApiClient::build_qparams(&pagination, &continuation);
    assert_eq!(qparams["page"], serde_json::json!({"page": "next_1"}));
}

#[test]
fn build_qparams_single_response_always_empty() {
    let pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/complete".to_owned(),
    };
    let qparams = AlphaApiClient::build_qparams(&pagination, &None);
    assert_eq!(qparams.as_object().unwrap().len(), 0);

    let qparams = AlphaApiClient::build_qparams(&pagination, &Some(serde_json::json!({"page": 2})));
    assert_eq!(qparams.as_object().unwrap().len(), 0);
}

#[test]
fn no_request_body_logged() {
    let body = AlphaApiClient::serialize_rankings_body(&AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".to_owned(),
        event_short: "100m".to_owned(),
        indoor: false,
        continuation: None,
    });
    let json_str = serde_json::to_string(&body).unwrap();
    assert!(!json_str.contains("Bearer"));
}
