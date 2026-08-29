use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;
use crate::alpha_model_raw::RawRankingsResponse;

fn make_single_response_client(server_url: &str, pointer: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: pointer.to_owned() },
        allowed_routes: vec![],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    })
    .expect("client creation must not fail")
}

fn make_next_page_client(server_url: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        },
        allowed_routes: vec![],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    })
    .expect("client creation must not fail")
}


// --- SingleResponse completeness ---

#[test]
fn single_response_complete_pointer_true() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": true}"#).unwrap();
    assert!(client.check_completeness(&raw).unwrap(), "complete=true => true");
}

#[test]
fn single_response_complete_pointer_false() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": false}"#).unwrap();
    assert!(!client.check_completeness(&raw).unwrap(), "complete=false => false");
}

#[test]
fn single_response_complete_pointer_missing() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": []}"#).unwrap();
    // Missing pointer => error (fail closed)
    assert!(client.check_completeness(&raw).is_err(), "missing pointer => error");
}

#[test]
fn single_response_complete_pointer_wrong_type() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": "yes"}"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "wrong type => error");
}

#[test]
fn single_response_complete_with_unknown_field() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": true, "unknown": 42}"#).unwrap();
    assert!(client.check_completeness(&raw).unwrap(), "complete=true with unknown field => true");
}

#[test]
fn single_response_complete_with_nested_pointer() {
    let client = make_single_response_client("https://example.com", "/settings/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "settings": {"complete": true}}"#).unwrap();
    assert!(client.check_completeness(&raw).unwrap(), "nested pointer works");
}
#[test]
fn single_response_continuation_complete_false_no_next_errors() {
    // continuation.complete=false in SingleResponse without nextPage => Incomplete.
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{ "groupedRankings": [], "complete": true, "continuation": {"page": 1, "complete": false} }"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "continuation.complete=false without nextPage => Incomplete");
}
#[test]
fn single_response_continuation_complete_false_with_next_page() {
    // SingleResponse continuation.complete=false with nextPage => Incomplete error.
    // SingleResponse cannot produce a continuation token, so this must fail closed.
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{ "groupedRankings": [], "complete": true, "nextPage": "2", "continuation": {"page": 1, "complete": false} }"#).unwrap();
    let err = client.check_completeness(&raw).expect_err("SingleResponse with incomplete continuation must fail closed");
    assert!(matches!(err, crate::alpha_api::AlphaApiError::Incomplete(_)), "expected Incomplete error, got {:?}", err);
}

#[test]
fn single_response_has_more_true_with_null_next_page_errors() {
    // SingleResponse must still reject hasMore=true without valid nextPage.
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": null}"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "hasMore=true with null nextPage must error in SingleResponse");
}

#[test]
fn single_response_has_more_true_with_empty_next_page_errors() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": ""}"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "hasMore=true with empty nextPage must error in SingleResponse");
}
#[test]
fn single_response_has_more_wrong_type_string_returns_error() {
    // hasMore:"yes" (wrong type) must be treated as malformed, not absent.
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": "yes"}"#).unwrap();
    let err = client.check_completeness(&raw).expect_err("hasMore wrong-type must return Incomplete");
    assert!(matches!(err, crate::alpha_api::AlphaApiError::Incomplete(_)));
}

#[test]
fn single_response_has_more_null_returns_error() {
    // hasMore:null must be treated as malformed, not absent.
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": null}"#).unwrap();
    let err = client.check_completeness(&raw).expect_err("hasMore null must return Incomplete");
    assert!(matches!(err, crate::alpha_api::AlphaApiError::Incomplete(_)));
}

#[test]
fn single_response_has_more_wrong_type_object_returns_error() {
    // hasMore:{...} (wrong type) must be treated as malformed, not absent.
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": {"foo": 1}}"#).unwrap();
    let err = client.check_completeness(&raw).expect_err("hasMore wrong-type object must return Incomplete");
    assert!(matches!(err, crate::alpha_api::AlphaApiError::Incomplete(_)));
}
// --- NextPage completeness ---

#[test]
fn nextpage_complete_when_has_more_false() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": false}"#).unwrap();
    assert!(client.check_completeness(&raw).unwrap(), "has_more=false => complete");
}

#[test]
fn nextpage_error_when_has_more_true_no_next() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": null}"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "has_more=true without next => error");
}

#[test]
fn nextpage_error_when_has_more_true_empty_next() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": ""}"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "empty next page => error");
}

#[test]
fn nextpage_incomplete_when_has_more_true_valid_next() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": "2"}"#).unwrap();
    let result = client.check_completeness(&raw);
    assert!(matches!(result, Ok(false)), "has_more=true with valid next page = incomplete");
}

#[test]
fn nextpage_continuation_complete_false_overrides_has_more_false() {
    // continuation.complete=false overrides hasMore=false.
    // Without usable next token => Incomplete (fail closed).
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{ "groupedRankings": [], "hasMore": false, "continuation": {"page": 1, "complete": false} }"#).unwrap();
    assert!(client.check_completeness(&raw).is_err(), "continuation.complete=false, hasMore=false, no next => Incomplete");
}

#[test]
fn nextpage_continuation_complete_false_with_next_token() {
    // continuation.complete=false with usable next token => Ok(false).
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{ "groupedRankings": [], "hasMore": true, "nextPage": "2", "continuation": {"page": 1, "complete": false} }"#).unwrap();
    assert!(!client.check_completeness(&raw).unwrap(), "continuation.complete=false with valid next => Ok(false)");
}


// --- JSON pointer navigation ---

#[test]
fn json_pointer_walk_nested() {
    let value = serde_json::json!({ "groupedRankings": [[{"AthleteID": 1}]], "page": 1, "complete": true });
    let val = AlphaApiClient::walk_pointer_value(&value, "/complete");
    assert_eq!(val, Some(&serde_json::json!(true)));
}

#[test]
fn json_pointer_walk_missing() {
    let value = serde_json::json!({ "groupedRankings": [], "page": 1 });
    let val = AlphaApiClient::walk_pointer_value(&value, "/nonexistent");
    assert!(val.is_none());
}

#[test]
fn json_pointer_walk_array() {
    let value = serde_json::json!({ "results": [{"id": 1}, {"id": 2}, {"id": 3}] });
    let val = AlphaApiClient::walk_pointer_value(&value, "/results/1/id");
    assert_eq!(val, Some(&serde_json::json!(2)));
}

#[test]
fn json_pointer_walk_escaped_key() {
    let value = serde_json::json!({ "a~b/c": "value" });
    let val = AlphaApiClient::walk_pointer_value(&value, "/a~0b~1c");
    assert_eq!(val, Some(&serde_json::json!("value")));
}
