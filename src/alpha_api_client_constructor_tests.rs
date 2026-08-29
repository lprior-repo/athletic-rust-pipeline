use crate::alpha_api::AlphaApiError;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;

fn make_base_config() -> crate::alpha_api::AlphaApiClientConfig {
    crate::alpha_api::AlphaApiClientConfig {
        base_url: "https://example.com".into(),
        rankings_path: "/rankings".into(),
        nav_info_path: "/nav".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".into(),
        },
        allowed_routes: vec!["/rankings".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        max_body_bytes: 8 * 1024 * 1024,
        cap_markers: vec![],
        auth_enabled: true,
        permission_reference: "test-ref".into(),
        min_delay_ms: 0,
        max_retry_delay_ms: 30_000,
    }
}

fn make_next_page_config() -> crate::alpha_api::AlphaApiClientConfig {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::NextPage {
        has_more_pointer: "/data/has_more".into(),
        next_page_pointer: "/data/next_page".into(),
        request_page_key: "page_token".into(),
    };
    config
}

fn expect_invalid_config(result: Result<AlphaApiClient, AlphaApiError>) -> AlphaApiError {
    match result {
        Ok(_) => panic!("expected InvalidConfig error, got Ok"),
        Err(e) => e,
    }
}

#[test]
fn constructor_rejects_empty_complete_pointer() {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::SingleResponse {
        complete_pointer: "".into(),
    };
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("complete_pointer"));
}

#[test]
fn constructor_rejects_relative_complete_pointer() {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::SingleResponse {
        complete_pointer: "complete".into(),
    };
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("start with"));
}

#[test]
fn constructor_rejects_trailing_tilde_in_complete_pointer() {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/data/complete~".into(),
    };
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("trailing"));
}

#[test]
fn constructor_rejects_invalid_tilde_escape_complete_pointer() {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/data/complete~2".into(),
    };
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("invalid RFC6901"));
}

#[test]
fn constructor_accepts_valid_complete_pointer() {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/data/rankings/complete".into(),
    };
    assert!(AlphaApiClient::new(config).is_ok());
}

#[test]
fn constructor_accepts_complete_pointer_with_valid_escapes() {
    let mut config = make_base_config();
    config.pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/data/~0~1/complete".into(),
    };
    assert!(AlphaApiClient::new(config).is_ok());
}

#[test]
fn constructor_rejects_empty_has_more_pointer() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut has_more_pointer,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    has_more_pointer.clear();
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("has_more_pointer"));
}

#[test]
fn constructor_rejects_relative_has_more_pointer() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut has_more_pointer,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    *has_more_pointer = "has_more".into();
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("has_more_pointer"));
}

#[test]
fn constructor_rejects_invalid_escape_has_more_pointer() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut has_more_pointer,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    *has_more_pointer = "/data/hm~x".into();
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("invalid RFC6901"));
}

#[test]
fn constructor_rejects_empty_next_page_pointer() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut next_page_pointer,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    next_page_pointer.clear();
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("next_page_pointer"));
}

#[test]
fn constructor_rejects_invalid_escape_next_page_pointer() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut next_page_pointer,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    *next_page_pointer = "/data/next~garbage".into();
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("invalid RFC6901"));
}

#[test]
fn constructor_rejects_empty_request_page_key() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut request_page_key,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    request_page_key.clear();
    let err = expect_invalid_config(AlphaApiClient::new(config));
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("request_page_key"));
}

#[test]
fn constructor_accepts_valid_next_page_config() {
    let config = make_next_page_config();
    assert!(AlphaApiClient::new(config).is_ok());
}

#[test]
fn constructor_accepts_next_page_with_valid_escapes() {
    let mut config = make_next_page_config();
    let PaginationConfig::NextPage {
        ref mut has_more_pointer,
        ref mut next_page_pointer,
        ..
    } = config.pagination
    else {
        unreachable!()
    };
    *has_more_pointer = "/data/~1more".into();
    *next_page_pointer = "/data/~0page".into();
    assert!(AlphaApiClient::new(config).is_ok());
}