#[cfg(test)]
mod tests {
    use crate::alpha_config_test_helpers::valid_config;

    // ---- original auth tests ----

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

    // ---- split from auth_route_tests ----

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
    fn max_retry_delay_below_min_delay_rejected() {
        let mut config = valid_config();
        config.authorization.min_delay_ms = 5000;
        config.authorization.max_retry_delay_ms = 1000;
        let error = config.validate().expect_err("max_retry_delay_ms < min_delay_ms must fail");
        assert!(
            error.to_string().contains("max_retry_delay_ms"),
            "error: {}",
            error
        );
        assert!(
            error.to_string().contains("min_delay_ms"),
            "error must mention min_delay_ms: {}",
            error
        );
    }

    #[test]
    fn max_retry_delay_exceeds_operational_ceiling_rejected() {
        let mut config = valid_config();
        config.authorization.max_retry_delay_ms = 500_000;
        let error = config.validate().expect_err("max_retry_delay_ms > ceiling must fail");
        assert!(
            error.to_string().contains("300_000") || error.to_string().contains("300000"),
            "error must mention ceiling: {}",
            error
        );
    }
}
