use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

use crate::alpha_model::{AlphaApiConfig, AuthorizationConfig, PaginationConfig};

/// Top-level alpha configuration combining authorization and API sections.
#[derive(Debug, Clone, Deserialize)]
pub struct AlphaConfig {
    pub authorization: AuthorizationConfig,
    pub api: AlphaApiConfig,
}

impl AlphaConfig {
    /// Load and validate an alpha configuration file.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading alpha configuration {}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).context("parsing alpha TOML configuration")?;
        config.validate()?;
        Ok(config)
    }

    /// Validate authorization and API settings; bail on any violation.
    pub fn validate(&self) -> Result<()> {
        self.validate_authorization()?;
        self.validate_api()?;
        Ok(())
    }

    fn validate_authorization(&self) -> Result<()> {
        let auth = &self.authorization;

        if auth.enabled && auth.permission_reference.is_empty() {
            bail!("authorization.enabled requires a non-empty permission_reference");
        }

        if auth.allowed_routes.is_empty() {
            bail!("authorization.allowed_routes must contain at least one route");
        }

        // Validate state codes: exactly 50 canonical US state codes, no duplicates.
        let canonical_states = [
            "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL",
            "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT",
            "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI",
            "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
        ];
        let canonical_set: std::collections::HashSet<&str> =
            canonical_states.iter().copied().collect();

        let mut seen_states: std::collections::HashSet<&str> =
            std::collections::HashSet::new();
        for code in &auth.allowed_states {
            if !canonical_set.contains(code.as_str()) {
                bail!(
                    "authorization.allowed_states contains unknown code '{}'; must be one of 50 canonical US states",
                    code
                );
            }
            if !seen_states.insert(code) {
                bail!("authorization.allowed_states contains duplicate code '{}'", code);
            }
        }
        if auth.allowed_states.len() != canonical_states.len() {
            bail!(
                "authorization.allowed_states must contain exactly 50 canonical state codes (found {})",
                auth.allowed_states.len()
            );
        }

        if auth.allowed_seasons.is_empty() {
            bail!("authorization.allowed_seasons must contain at least one season");
        }

        if auth.allowed_genders.is_empty() {
            bail!("authorization.allowed_genders must contain at least one gender");
        }

        if auth.max_concurrent_requests != 1 {
            bail!(
                "authorization.max_concurrent_requests must be exactly 1 for the sequential first implementation"
            );
        }

        if auth.min_delay_ms < 500 {
            bail!("authorization.min_delay_ms must be at least 500 ms");
        }

        Ok(())
    }

    fn validate_api(&self) -> Result<()> {
        let api = &self.api;

        // Base URL must be a valid HTTPS URL with nonempty host.
        let parsed = url::Url::parse(&api.base_url)
            .with_context(|| format!("api.base_url is not a valid URL (got '{}')", api.base_url))?;
        if parsed.scheme() != "https" {
            bail!(
                "api.base_url must use https scheme (got '{}')",
                api.base_url
            );
        }
        if parsed.host_str().map_or(true, str::is_empty) {
            bail!(
                "api.base_url must have a nonempty host (got '{}')",
                api.base_url
            );
        }

        // Rankings and nav paths must be HTTPS-relative (path-only, no scheme/host/query/fragment).
        let allowed = &self.authorization.allowed_routes;
        for route in [api.rankings_path.as_str(), api.nav_info_path.as_str()] {
            // Must be an absolute path starting with /.
            if !route.starts_with('/') {
                bail!(
                    "api route '{}' must be an HTTPS-relative path starting with '/'",
                    route
                );
            }
            // Reject routes that contain scheme, host, query, or fragment markers.
            // Reject network-path references (//host/path) and other scheme/host/query/fragment.
            if route.contains("//") || route.contains('@') || route.contains('?') || route.contains('#') {
                bail!(
                    "api route '{}' must not contain scheme, host, query, or fragment",
                    route
                );
            }
            if !allowed.contains(&route.to_string()) {
                bail!(
                    "api route '{}' is not listed in authorization.allowed_routes",
                    route
                );
            }
        }

        // Timeout must be bounded (1..=300).
        if api.timeout_seconds < 1 || api.timeout_seconds > 300 {
            bail!("api.timeout_seconds must be between 1 and 300");
        }

        // Max retries bounded (0..=5).
        if api.max_retries > 5 {
            bail!("api.max_retries must be at most 5");
        }

        // Pagination pointers must be non-empty.
        self.validate_pagination()?;

        // Profile enrichment: only allowed when explicitly authorized via allowed_profile_routes.
        if self.authorization.allow_profile_enrichment {
            if self.authorization.allowed_profile_routes.is_empty() {
                bail!(
                    "authorization.allow_profile_enrichment requires non-empty allowed_profile_routes"
                );
            }
            for profile_route in &self.authorization.allowed_profile_routes {
                // Validate each profile route is HTTPS-relative (path-only).
                if !profile_route.starts_with('/') {
                    bail!(
                        "allowed_profile_routes entry '{}' must start with '/'",
                        profile_route
                    );
                }
                if profile_route.contains("//") || profile_route.contains('@') || profile_route.contains('?') || profile_route.contains('#') {
                    bail!(
                        "allowed_profile_routes entry '{}' must not contain scheme, host, query, or fragment",
                        profile_route
                    );
                }
                if !allowed.contains(profile_route) {
                    bail!(
                        "allowed_profile_routes entry '{}' is not in authorization.allowed_routes",
                        profile_route
                    );
                }
            }
        }

        Ok(())
    }

    fn validate_pagination(&self) -> Result<()> {
        match &self.api.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                if complete_pointer.is_empty() {
                    bail!("api.pagination.complete_pointer must be non-empty for single_response mode");
                }
            }
            PaginationConfig::NextPage {
                has_more_pointer,
                next_page_pointer,
                request_page_key,
            } => {
                if has_more_pointer.is_empty() {
                    bail!("api.pagination.has_more_pointer must be non-empty for next_page mode");
                }
                if next_page_pointer.is_empty() {
                    bail!("api.pagination.next_page_pointer must be non-empty for next_page mode");
                }
                if request_page_key.is_empty() {
                    bail!("api.pagination.request_page_key must be non-empty for next_page mode");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn valid_config() -> AlphaConfig {
        AlphaConfig {
            authorization: AuthorizationConfig {
                enabled: false,
                permission_reference: "disabled-example-no-permission".to_owned(),
                allowed_routes: vec![
                    "/api/v1/tfRankings/GetRankings".to_owned(),
                    "/api/v1/tfRankings/GetNavInfo".to_owned(),
                ],
                allowed_sports: vec!["Track and Field".to_owned()],
                allowed_states: canonical_states(),
                allowed_seasons: vec![2026],
                allowed_genders: vec!["m".to_owned(), "f".to_owned()],
                allowed_fields: vec![
                    "AthleteID".to_owned(),
                    "AthleteName".to_owned(),
                    "GradeID".to_owned(),
                    "TeamName".to_owned(),
                    "State".to_owned(),
                    "MeetID".to_owned(),
                    "IDResult".to_owned(),
                    "EventShort".to_owned(),
                    "Measure".to_owned(),
                    "ResultDate".to_owned(),
                    "SeasonID".to_owned(),
                ],
                allowed_profile_routes: vec![],
                allow_profile_enrichment: false,
                max_concurrent_requests: 1,
                min_delay_ms: 750,
            },
            api: AlphaApiConfig {
                base_url: "https://www.athletic.net".to_owned(),
                rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
                nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
                timeout_seconds: 30,
                max_retries: 2,
                pagination: PaginationConfig::SingleResponse {
                    complete_pointer: "/settings/complete".to_owned(),
                },
            },
        }
    }

    fn canonical_states() -> Vec<String> {
        [
            "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL",
            "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT",
            "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI",
            "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect()
    }

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
            error.to_string().contains("not in authorization.allowed_routes"),
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
            error.to_string().contains("host"),
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
            error.to_string().contains("start with '/'") || error.to_string().contains("host"),
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
            error.to_string().contains("start with '/'"),
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
