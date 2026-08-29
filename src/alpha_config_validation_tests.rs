#[cfg(test)]
mod tests {
    use crate::alpha_config::test_helpers::valid_config;
    use crate::alpha_model::PaginationConfig;



    // ---- validation failure tests ----

    #[test]
    fn enabled_alpha_requires_permission_reference() {
        let mut config = valid_config();
        config.authorization.enabled = true;
        config.authorization.permission_reference.clear();
        let error = config.validate().expect_err("missing permission must fail");
        assert!(
            error.to_string().contains("permission_reference"),
            "error: {}",
            error
        );
    }

    #[test]
    fn empty_allowed_states_rejected() {
        let mut config = valid_config();
        config.authorization.allowed_states.clear();
        let error = config.validate().expect_err("empty states must fail");
        assert!(error.to_string().contains("50"), "error: {}", error);
    }

    #[test]
    fn duplicate_state_code_rejected() {
        let mut config = valid_config();
        config
            .authorization
            .allowed_states
            .push("CA".to_owned());
        let error = config.validate().expect_err("duplicate state must fail");
        assert!(error.to_string().contains("duplicate"), "error: {}", error);
    }

    #[test]
    fn unknown_state_code_rejected() {
        let mut config = valid_config();
        // Replace one state with an invalid code.
        config.authorization.allowed_states[0] = "XX".to_owned();
        let error = config.validate().expect_err("unknown state must fail");
        assert!(
            error.to_string().contains("unknown code"),
            "error: {}",
            error
        );
    }

    #[test]
    fn non_https_base_url_rejected() {
        let mut config = valid_config();
        config.api.base_url = "http://example.com".to_owned();
        let error = config.validate().expect_err("non-HTTPS base must fail");
        assert!(
            error.to_string().contains("https"),
            "error: {}",
            error
        );
    }

    #[test]
    fn unauthorized_route_rejected() {
        let mut config = valid_config();
        config.api.rankings_path = "/api/v1/other".to_owned();
        let error = config.validate().expect_err("unauthorized route must fail");
        assert!(
            error.to_string().contains("allowed_routes"),
            "error: {}",
            error
        );
    }

    #[test]
    fn profile_enrichment_without_profile_route_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        let error = config
            .validate()
            .expect_err("profile enrichment without profile route must fail");
        assert!(
            error.to_string().contains("allowed_profile_routes"),
            "error: {}",
            error
        );
    }

    #[test]
    fn concurrency_not_one_rejected() {
        let mut config = valid_config();
        config.authorization.max_concurrent_requests = 2;
        let error = config.validate().expect_err("concurrency != 1 must fail");
        assert!(
            error.to_string().contains("exactly 1"),
            "error: {}",
            error
        );
    }

    #[test]
    fn min_delay_too_small_rejected() {
        let mut config = valid_config();
        config.authorization.min_delay_ms = 100;
        let error = config.validate().expect_err("delay < 500 must fail");
        assert!(
            error.to_string().contains("500"),
            "error: {}",
            error
        );
    }

    #[test]
    fn empty_complete_pointer_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse {
            complete_pointer: "".to_owned(),
        };
        let error = config.validate().expect_err("empty complete_pointer must fail");
        assert!(
            error.to_string().contains("complete_pointer"),
            "error: {}",
            error
        );
    }
    #[test]
    fn empty_allowed_routes_rejected() {
        let mut config = valid_config();
        config.authorization.allowed_routes.clear();
        let error = config.validate().expect_err("empty allowed_routes must fail");
        assert!(
            error.to_string().contains("allowed_routes"),
            "error: {}",
            error
        );
    }
    #[test]
    fn invalid_base_url_not_valid() {
        let mut config = valid_config();
        config.api.base_url = "not-a-url".to_owned();
        let error = config.validate().expect_err("invalid base_url must fail");
        assert!(error.to_string().contains("not a valid URL"), "error: {}", error);
    }

    #[test]
    fn base_url_missing_host_rejected() {
        let mut config = valid_config();
        config.api.base_url = "https://".to_owned();
        let error = config.validate().expect_err("base_url missing host must fail");
        assert!(error.to_string().contains("not a valid URL") || error.to_string().contains("nonempty host"), "error: {}", error);
    }

    #[test]
    fn route_with_scheme_rejected() {
        let mut config = valid_config();
        config.api.rankings_path = "https://evil.com/api/v1/tfRankings/GetRankings".to_owned();
        let error = config.validate().expect_err("route with scheme must fail");
        assert!(
            error.to_string().contains("starting with '/'"),
            "error: {}",
            error
        );
    }

    #[test]
    fn route_with_query_rejected() {
        let mut config = valid_config();
        config.api.rankings_path = "/api/v1/tfRankings/GetRankings?debug=1".to_owned();
        let error = config.validate().expect_err("route with query must fail");
        assert!(
            error.to_string().contains("query"),
            "error: {}",
            error
        );
    }

    #[test]
    fn new_returns_ok_with_valid_config() {
        // Regression: AlphaApiClient::new must not panic with valid config.
        let config = crate::alpha_api::AlphaApiClientConfig {
            base_url: "https://example.com".into(),
            rankings_path: "/api/v1/tfRankings/GetRankings".into(),
            nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
            timeout_seconds: 30,
            max_retries: 0,
            pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
            allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
            allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
            max_concurrent_requests: 1,
            min_delay_ms: 0,
            cap_markers: vec![],
        };
        let result = crate::alpha_api_client::AlphaApiClient::new(config);
        assert!(result.is_ok(), "new() must return Ok with valid config");
    }
    #[test]
    fn bare_complete_pointer_rejected() {
        // RFC6901 pointers must be absolute (start with /).
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse { complete_pointer: "settings/complete".into() };
        let error = config.validate().expect_err("bare pointer must be rejected");
        assert!(error.to_string().contains("absolute RFC6901"), "error: {}", error);
    }
    #[test]
    fn bare_next_page_pointer_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".into(),
            next_page_pointer: "nextPage".into(),
            request_page_key: "page".into(),
        };
        let error = config.validate().expect_err("bare next_page_pointer must be rejected");
        assert!(error.to_string().contains("absolute RFC6901"), "error: {}", error);
    }
    #[test]
    fn empty_has_more_pointer_rejected_in_next_page_mode() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "".into(),
            next_page_pointer: "/nextPage".into(),
            request_page_key: "page".into(),
        };
        let error = config.validate().expect_err("empty has_more_pointer must be rejected");
        assert!(error.to_string().contains("non-empty"), "error: {}", error);
    }
    #[test]
    fn empty_complete_pointer_always_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse { complete_pointer: "".into() };
        let error = config.validate().expect_err("empty complete_pointer must be rejected in single_response mode");
        assert!(error.to_string().contains("non-empty"), "error: {}", error);
    }
}
