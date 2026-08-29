use crate::alpha_api::{validate_pagination_config, AlphaApiError};
use crate::alpha_model::PaginationConfig;
// --- validate_pagination_config unit tests ---

#[test]
fn validate_single_response_empty_complete_pointer() {
    let config = PaginationConfig::SingleResponse {
        complete_pointer: "".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("complete_pointer"));
}

#[test]
fn validate_single_response_relative_complete_pointer() {
    let config = PaginationConfig::SingleResponse {
        complete_pointer: "complete".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("start with"));
}

#[test]
fn validate_single_response_trailing_tilde() {
    let config = PaginationConfig::SingleResponse {
        complete_pointer: "/data/complete~".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("trailing"));
}

#[test]
fn validate_single_response_invalid_tilde_escape() {
    let config = PaginationConfig::SingleResponse {
        complete_pointer: "/data/complete~2".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("invalid RFC6901"));
}

#[test]
fn validate_single_response_valid_pointer() {
    let config = PaginationConfig::SingleResponse {
        complete_pointer: "/data/rankings/complete".into(),
    };
    assert!(validate_pagination_config(&config).is_ok());
}

#[test]
fn validate_single_response_valid_escapes() {
    let config = PaginationConfig::SingleResponse {
        complete_pointer: "/data/~0~1/complete".into(),
    };
    assert!(validate_pagination_config(&config).is_ok());
}

// --- NextPage: has_more_pointer validation ---

#[test]
fn validate_next_page_empty_has_more_pointer() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "".into(),
        next_page_pointer: "/data/next_page".into(),
        request_page_key: "page_token".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("has_more_pointer"));
}

#[test]
fn validate_next_page_relative_has_more_pointer() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "has_more".into(),
        next_page_pointer: "/data/next_page".into(),
        request_page_key: "page_token".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("has_more_pointer"));
}

#[test]
fn validate_next_page_invalid_escape_has_more_pointer() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "/data/hm~x".into(),
        next_page_pointer: "/data/next_page".into(),
        request_page_key: "page_token".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("invalid RFC6901"));
}

// --- NextPage: next_page_pointer validation ---

#[test]
fn validate_next_page_empty_next_page_pointer() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "/data/has_more".into(),
        next_page_pointer: "".into(),
        request_page_key: "page_token".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("next_page_pointer"));
}

#[test]
fn validate_next_page_invalid_escape_next_page_pointer() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "/data/has_more".into(),
        next_page_pointer: "/data/next~garbage".into(),
        request_page_key: "page_token".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("invalid RFC6901"));
}

// --- NextPage: request_page_key validation ---

#[test]
fn validate_next_page_empty_request_page_key() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "/data/has_more".into(),
        next_page_pointer: "/data/next_page".into(),
        request_page_key: "".into(),
    };
    let err = validate_pagination_config(&config).unwrap_err();
    assert!(matches!(err, AlphaApiError::InvalidConfig(_)));
    assert!(err.to_string().contains("request_page_key"));
}

#[test]
fn validate_next_page_all_valid() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "/data/has_more".into(),
        next_page_pointer: "/data/next_page".into(),
        request_page_key: "page_token".into(),
    };
    assert!(validate_pagination_config(&config).is_ok());
}

#[test]
fn validate_next_page_valid_escapes() {
    let config = PaginationConfig::NextPage {
        has_more_pointer: "/data/~1more".into(),
        next_page_pointer: "/data/~0page".into(),
        request_page_key: "page_token".into(),
    };
    assert!(validate_pagination_config(&config).is_ok());
}