#[cfg(test)]
mod tests {
    use crate::alpha_config::test_helpers::valid_config;
    use crate::alpha_model::PaginationConfig;

    use crate::alpha_config::AlphaConfig;
    use std::path::Path;

    #[test]
    fn empty_seasons_rejected() {
        let mut config = valid_config();
        config.authorization.allowed_seasons.clear();
        let error = config.validate().expect_err("empty seasons must fail");
        assert!(
            error.to_string().contains("at least one season"),
            "error: {}",
            error
        );
    }

    #[test]
    fn empty_genders_rejected() {
        let mut config = valid_config();
        config.authorization.allowed_genders.clear();
        let error = config.validate().expect_err("empty genders must fail");
        assert!(
            error.to_string().contains("at least one gender"),
            "error: {}",
            error
        );
    }

    #[test]
    fn timeout_too_large_rejected() {
        let mut config = valid_config();
        config.api.timeout_seconds = 400;
        let error = config.validate().expect_err("timeout > 300 must fail");
        assert!(
            error.to_string().contains("between 1 and 300"),
            "error: {}",
            error
        );
    }

    #[test]
    fn retries_too_large_rejected() {
        let mut config = valid_config();
        config.api.max_retries = 10;
        let error = config.validate().expect_err("retries > 5 must fail");
        assert!(
            error.to_string().contains("at most 5"),
            "error: {}",
            error
        );
    }

    #[test]
    fn next_page_empty_pointers_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "".to_owned(),
            next_page_pointer: "/page".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let error = config.validate().expect_err("empty has_more_pointer must fail");
        assert!(
            error.to_string().contains("has_more_pointer"),
            "error: {}",
            error
        );
    }

    #[test]
    fn profile_enrichment_with_empty_allowed_profile_routes_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        // allowed_profile_routes is empty in valid_config
        let error = config
            .validate()
            .expect_err("enrichment with empty allowed_profile_routes must fail");
        assert!(
            error.to_string().contains("allowed_profile_routes"),
            "error: {}",
            error
        );
    }

    #[test]
    fn profile_enrichment_with_unauthorized_profile_route_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "/api/v1/other/Profile".to_owned(),
        ];
        let error = config
            .validate()
            .expect_err("unauthorized profile route must fail");
        assert!(
            error.to_string().contains("is not in"),
            "error: {}",
            error
        );
    }
    #[test]
    fn timeout_zero_rejected() {
        let mut config = valid_config();
        config.api.timeout_seconds = 0;
        let error = config.validate().expect_err("timeout 0 must fail");
        assert!(
            error.to_string().contains("between 1 and 300"),
            "error: {}",
            error
        );
    }

    #[test]
    fn next_page_empty_next_page_pointer_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "/more".to_owned(),
            next_page_pointer: "".to_owned(),
            request_page_key: "page".to_owned(),
        };
        let error = config.validate().expect_err("empty next_page_pointer must fail");
        assert!(
            error.to_string().contains("next_page_pointer"),
            "error: {}",
            error
        );
    }

    #[test]
    fn next_page_empty_request_page_key_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "/more".to_owned(),
            next_page_pointer: "/page".to_owned(),
            request_page_key: "".to_owned(),
        };
        let error = config.validate().expect_err("empty request_page_key must fail");
        assert!(
            error.to_string().contains("request_page_key"),
            "error: {}",
            error
        );
    }

    #[test]
    fn route_network_path_rejected() {
        let mut config = valid_config();
        config.api.rankings_path = "//evil.com/api/v1/tfRankings/GetRankings".to_owned();
        let error = config.validate().expect_err("network-path route must fail");
        assert!(
            error.to_string().contains("network-path"),
            "error: {}",
            error
        );
    }

    #[test]
    fn route_same_host_network_path_rejected() {
        let mut config = valid_config();
        config.api.rankings_path = "//www.athletic.net/secret".to_owned();
        let error = config.validate().expect_err("same-host network-path route must fail");
        assert!(
            error.to_string().contains("network-path"),
            "error: {}",
            error
        );
    }

    #[test]
    fn profile_route_with_scheme_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "https://example.com/api/Profile".to_owned(),
        ];
        let error = config
            .validate()
            .expect_err("profile route with scheme must fail");
        assert!(
            error.to_string().contains("starting with '/'"),
            "error: {}",
            error
        );
    }

    #[test]
    fn profile_route_not_started_with_slash_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "api/v1/Profile".to_owned(),
        ];
        let error = config
            .validate()
            .expect_err("profile route without leading / must fail");
        assert!(
            error.to_string().contains("starting with '/'"),
            "error: {}",
            error
        );
    }

    // ---- success tests ----

    #[test]
    fn valid_config_passes_validation() {
        let config = valid_config();
        config.validate().expect("valid config should pass");
    }

    #[test]
    fn disabled_example_parses_correctly() {
        let config = AlphaConfig::load(Path::new(
            "alpha.example.toml",
        ))
        .expect("example file should parse");
        assert!(!config.authorization.enabled);
        assert_eq!(
            config.api.base_url,
            "https://www.athletic.net"
        );
        assert_eq!(config.authorization.max_concurrent_requests, 1);
        assert_eq!(config.authorization.min_delay_ms, 750);
        assert!(!config.authorization.allow_profile_enrichment);
        match &config.api.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                assert_eq!(complete_pointer, "/settings/complete");
            }
            _ => panic!("expected single_response mode"),
        }
    }
}
