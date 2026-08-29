#[cfg(test)]
mod tests {
    use crate::alpha_config_test_helpers::valid_config;
    use crate::alpha_model::PaginationConfig;

    #[test]
    fn cap_marker_empty_rejected() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["".to_owned()];
        let error = config.validate().expect_err("empty cap_marker must fail");
        assert!(
            error.to_string().contains("must be non-empty"),
            "error: {}",
            error
        );
    }

    #[test]
    fn cap_marker_top_level_key_ok() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["truncated".to_owned(), "has_more".to_owned()];
        config.validate().expect("top-level keys should pass");
    }

    #[test]
    fn cap_marker_valid_rfc6901_pointer_ok() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["/metadata/truncated".to_owned()];
        config.validate().expect("valid RFC6901 pointer should pass");
    }

    #[test]
    fn cap_marker_valid_escaped_pointer_ok() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["/data/value~0".to_owned()]; // ~0 encodes literal ~
        config.validate().expect("valid escaped RFC6901 pointer should pass");
    }

    #[test]
    fn cap_marker_malformed_path_rejected() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["/metadata/truncated~2".to_owned()];
        let error = config.validate().expect_err("malformed RFC6901 escape must fail");
        assert!(
            error.to_string().contains("invalid RFC6901"),
            "error: {}",
            error
        );
    }

    #[test]
    fn cap_marker_slash_in_top_level_rejected() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["metadata/truncated".to_owned()];
        let error = config
            .validate()
            .expect_err("slash in non-pointer marker must fail");
        assert!(
            error.to_string().contains("not a valid RFC6901 pointer"),
            "error: {}",
            error
        );
    }

    #[test]
    fn cap_marker_tilde_in_top_level_rejected() {
        let mut config = valid_config();
        config.api.cap_markers = vec!["truncated~bad".to_owned()];
        let error = config
            .validate()
            .expect_err("tilde in non-pointer marker must fail");
        assert!(
            error.to_string().contains("not a valid RFC6901 pointer"),
            "error: {}",
            error
        );
    }

    #[test]
    fn empty_complete_pointer_always_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::SingleResponse { complete_pointer: "".into() };
        let error = config.validate().expect_err("empty complete_pointer must be rejected in single_response mode");
        assert!(
            error.to_string().contains("complete_pointer"),
            "error: {}",
            error
        );
    }

    #[test]
    fn profile_enrichment_with_empty_allowed_profile_routes_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "/api/v1/other/Profile".to_owned(),
        ];
        let error = config.validate().expect_err("enrichment without route must fail");
        assert!(
            error.to_string().contains("not in allowed_routes"),
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
            error.to_string().contains("between 1"),
            "error: {}",
            error
        );
    }
    #[test]
    fn max_retries_too_high_rejected() {
        let mut config = valid_config();
        config.api.max_retries = 10;
        let error = config.validate().expect_err("max_retries > 5 must fail");
        assert!(
            error.to_string().contains("at most 5"),
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
        let error = config.validate().expect_err("profile route without / must fail");
        assert!(
            error.to_string().contains("starting with"),
            "error: {}",
            error
        );
    }
    #[test]
    fn profile_route_with_backslash_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "/api/v1/\\evil.com/Profile".to_owned(),
        ];
        let error = config.validate().expect_err("profile route with \\ must fail");
        assert!(
            error.to_string().contains("backslash"),
            "error: {}",
            error
        );
    }
    #[test]
    fn profile_route_with_query_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "/api/v1/Profile?debug=1".to_owned(),
        ];
        let error = config.validate().expect_err("profile route with query must fail");
        assert!(
            error.to_string().contains("query"),
            "error: {}",
            error
        );
    }
    #[test]
    fn profile_route_same_host_different_port_rejected() {
        let mut config = valid_config();
        config.api.base_url = "https://athletic.net".to_owned();
        config.api.rankings_path = "/api/v1/tfRankings/GetRankings".to_owned();
        config.api.nav_info_path = "/api/v1/tfRankings/GetNavInfo".to_owned();
        config.authorization.allowed_routes = vec![
            "/api/v1/tfRankings/GetRankings".to_owned(),
            "/api/v1/tfRankings/GetNavInfo".to_owned(),
        ];
        config.api.rankings_path = "/api/v1/tfRankings/GetRankings:443".to_owned();
        let error = config.validate().expect_err("invalid route with port must fail");
        assert!(
            error.to_string().contains("cannot be resolved") || error.to_string().contains("route"),
            "error: {}",
            error
        );
    }
    #[test]
    fn profile_route_different_host_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "//evil.com/api/Profile".to_owned(),
        ];
        let error = config.validate().expect_err("different host must fail");
        assert!(
            error.to_string().contains("network-path"),
            "error: {}",
            error
        );
    }
    #[test]
    fn profile_route_network_path_rejected() {
        let mut config = valid_config();
        config.authorization.allow_profile_enrichment = true;
        config.authorization.allowed_profile_routes = vec![
            "//other.athletic.net/api/Profile".to_owned(),
        ];
        let error = config.validate().expect_err("// must fail");
        assert!(
            error.to_string().contains("network-path"),
            "error: {}",
            error
        );
    }
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
    fn request_page_key_empty_rejected() {
        let mut config = valid_config();
        config.api.pagination = PaginationConfig::NextPage {
            has_more_pointer: "/data/hm".to_owned(),
            next_page_pointer: "/data/next".to_owned(),
            request_page_key: "".to_owned(),
        };
        let error = config.validate().expect_err("empty request_page_key must fail");
        assert!(
            error.to_string().contains("non-empty"),
            "error: {}",
            error
        );
    }

    #[test]
    fn valid_config_passes_validation() {
        let config = valid_config();
        config.validate().expect("valid config should pass");
    }
}
