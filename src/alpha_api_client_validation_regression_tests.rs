use crate::alpha_api::AlphaApiError;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;
#[test]
fn invalid_config_max_body_bytes_zero() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 0,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_max_body_bytes_over_8mi() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8_388_609,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_max_retries_over_5() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 6,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_timeout_zero() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 0, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_timeout_over_300() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 301, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_retry_delay_below_min() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 1000, max_retry_delay_ms: 500,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_retry_delay_over_300k() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 300_001,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}
#[test]
fn invalid_config_retry_delay_zero() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 1000, max_retry_delay_ms: 0,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}
