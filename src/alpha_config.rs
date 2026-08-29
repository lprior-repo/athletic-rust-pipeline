use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

use crate::alpha_model::{AlphaApiConfig, AuthorizationConfig, PaginationConfig};
use crate::alpha_route_validation::validate_route;

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
        let mut seen_states = std::collections::HashSet::new();
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
        if parsed.host_str().is_none_or(str::is_empty) {
            bail!(
                "api.base_url must have a nonempty host (got '{}')",
                api.base_url
            );
        }
        let allowed = &self.authorization.allowed_routes;

        // Validate every API and allowed route by resolving against base URL.
        // Reject backslash authority escapes, non-HTTPS schemes, different hosts,
        for route in [api.rankings_path.as_str(), api.nav_info_path.as_str()] {
            validate_route(route, &parsed, allowed)?;
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
                validate_route(profile_route, &parsed, allowed)?;
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
pub(crate) mod test_helpers {
    use super::*;

    pub(crate) fn valid_config() -> AlphaConfig {
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

    pub(crate) fn canonical_states() -> Vec<String> {
        [
            "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN",
            "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV",
            "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN",
            "TX", "UT", "VT", "VA", "WA", "WV", "WI", "WY",
        ]
        .into_iter()
        .map(|s| s.to_owned())
        .collect()
    }
}

