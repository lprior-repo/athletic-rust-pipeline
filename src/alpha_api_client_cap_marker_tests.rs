use crate::alpha_api::AlphaApiError;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;

fn make_client_with_cap_markers(cap_markers: Vec<String>) -> Result<AlphaApiClient, AlphaApiError> {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "https://example.com".into(),
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".into(),
        },
        allowed_routes: vec![
            "/api/v1/tfRankings/GetRankings".into(),
            "/api/v1/tfRankings/GetNavInfo".into(),
        ],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers,
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test-permission".into(),
    })
}

#[test]
fn new_rejects_empty_cap_marker() {
    let result = make_client_with_cap_markers(vec!["".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_malformed_rfc6901_tilde_escape() {
    let result = make_client_with_cap_markers(vec!["/bad~2".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_trailing_tilde_in_pointer() {
    let result = make_client_with_cap_markers(vec!["/path~".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_accepts_valid_escaped_pointer_tilde0() {
    let result = make_client_with_cap_markers(vec!["/a~0b".into()]);
    assert!(result.is_ok());
}

#[test]
fn new_accepts_valid_escaped_pointer_tilde1() {
    let result = make_client_with_cap_markers(vec!["/path~1key".into()]);
    assert!(result.is_ok());
}

#[test]
fn new_accepts_top_level_key_without_special_chars() {
    let result = make_client_with_cap_markers(vec!["rankings".into()]);
    assert!(result.is_ok());
}

#[test]
fn new_rejects_top_level_key_containing_slash() {
    let result = make_client_with_cap_markers(vec!["a/b".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_top_level_key_containing_tilde() {
    let result = make_client_with_cap_markers(vec!["a~b".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_empty_and_malformed_markers_in_list() {
    let result = make_client_with_cap_markers(vec!["valid".into(), "".into(), "/bad~2".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_accepts_multiple_valid_markers() {
    let result = make_client_with_cap_markers(vec![
        "rankings".into(),
        "/data".into(),
        "/a~0b".into(),
        "/path~1key".into(),
    ]);
    assert!(result.is_ok());
}
