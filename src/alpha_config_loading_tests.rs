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
}
