use crate::alpha_model::AlphaRequest;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;
use crate::alpha_api::AlphaApiError;
// --- Request serialization ---
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
    let qparams = AlphaApiClient::build_qparams(&pagination, &None).unwrap();
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
    let qparams = AlphaApiClient::build_qparams(&pagination, &continuation).unwrap();
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
        "gender", "qualifyingListKey", "version", "debug", "qParams",
    ];
    assert_eq!(body.as_object().unwrap().len(), 10, "body must have exactly 10 keys");
    for key in &expected_keys {
        assert!(body.get(*key).is_some(), "body must contain key '{}'", key);
    }
    assert!(body["qParams"].is_object(), "qParams must be an object");
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
    #[test]
    fn build_qparams_nextpage_rejects_null_continuation() {
        let pagination = PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let continuation = Some(serde_json::Value::Null);
        let err = AlphaApiClient::build_qparams(&pagination, &continuation).unwrap_err();
        assert!(matches!(err, AlphaApiError::Incomplete(msg) if msg.contains("null")));
    }
    #[test]
    fn build_qparams_nextpage_rejects_empty_string_continuation() {
        let pagination = PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let continuation = Some(serde_json::Value::String("".into()));
        let err = AlphaApiClient::build_qparams(&pagination, &continuation).unwrap_err();
        assert!(matches!(err, AlphaApiError::Incomplete(msg) if msg.contains("empty")));
    }
    #[test]
    fn build_qparams_nextpage_rejects_empty_object_continuation() {
        let pagination = PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let continuation = Some(serde_json::json!({}));
        let err = AlphaApiClient::build_qparams(&pagination, &continuation).unwrap_err();
        assert!(matches!(err, AlphaApiError::Incomplete(msg) if msg.contains("empty object")));
    }

