#[cfg(test)]
mod tests {
    use crate::alpha_config_test_helpers::valid_config;



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
}
