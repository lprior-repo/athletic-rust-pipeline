#[cfg(test)]
mod tests {
    use crate::alpha_config::AlphaConfig;
    use crate::alpha_model::PaginationConfig;
    use std::path::Path;


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

    #[test]
    fn to_client_config_maps_all_fields() {
        let config = AlphaConfig::load(Path::new("alpha.example.toml")).expect("example file should parse");
        let client_config = config.to_client_config();
        assert_eq!(client_config.base_url, "https://www.athletic.net");
        assert_eq!(client_config.cap_markers, config.api.cap_markers, "cap_markers must map from api.cap_markers");
        assert!(!client_config.allowed_fields.is_empty(), "allowed_fields must map from authorization.allowed_fields");
        assert_eq!(client_config.timeout_seconds, 30);
        assert_eq!(client_config.max_retries, 2);
        assert_eq!(client_config.min_delay_ms, 750);
        assert_eq!(client_config.max_concurrent_requests, 1);
        // Verify pagination was wired through
        match &client_config.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                assert_eq!(complete_pointer, "/settings/complete");
            }
            _ => panic!("expected single_response pagination"),
        }
    }
}
