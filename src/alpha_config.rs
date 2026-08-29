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

/// Validate a cap_marker: non-empty, either a top-level key (no /, no ~)
/// or a strict RFC6901 JSON pointer (starts with /, every ~ followed by 0 or 1).
fn validate_cap_marker(marker: &str, idx: usize) -> Result<()> {
    if marker.starts_with('/') {
        // RFC6901 pointer: validate escape sequences.
        let mut chars = marker.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '~' {
                let next = chars.peek().copied().unwrap_or('\0');
                if next != '0' && next != '1' {
                    bail!(
                        "api.cap_markers[{idx}] has invalid RFC6901 escape '~{next}' at position {}",
                        marker.find(ch).unwrap_or(0),
                    );
                }
            }
        }
    } else {
        // Top-level key: must not contain / or ~.
        if marker.contains('/') || marker.contains('~') {
            bail!(
                "api.cap_markers[{idx}] is not a top-level key and not a valid RFC6901 pointer: '{marker}'"
            );
        }
    }
    Ok(())
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

        // max_retry_delay_ms must be at least min_delay_ms and bounded.
        if auth.max_retry_delay_ms < auth.min_delay_ms {
            bail!(
                "authorization.max_retry_delay_ms ({}) must be >= min_delay_ms ({})",
                auth.max_retry_delay_ms,
                auth.min_delay_ms
            );
        }
        const MAX_RETRY_AFTER_MS: u64 = 300_000;
        if auth.max_retry_delay_ms > MAX_RETRY_AFTER_MS {
            bail!("authorization.max_retry_delay_ms must be at most {} ms", MAX_RETRY_AFTER_MS);
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
        // Validate cap_markers: nonempty, either top-level key or strict RFC6901 pointer.
        for (idx, marker) in api.cap_markers.iter().enumerate() {
            if marker.is_empty() {
                bail!("api.cap_markers[{idx}] must be non-empty");
            }
            validate_cap_marker(marker, idx)?;
        }

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
        let validate_absolute_ptr = |ptr: &str, name: &str| -> Result<()> {
            if !ptr.is_empty() && !ptr.starts_with('/') {
                bail!("api.pagination.{name} must be an absolute RFC6901 JSON pointer (starts with '/') or empty, got '{ptr}'");
            }
            // Validate RFC6901 escape sequences: every ~ must be followed by 0 or 1.
            let mut chars = ptr.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '~' {
                    let next = chars.peek().copied().unwrap_or('\0');
                    if next != '0' && next != '1' {
                        bail!(
                            "api.pagination.{name} has invalid RFC6901 escape '~{next}' at position {}",
                            ptr.find(ch).unwrap_or(0),
                        );
                    }
                }
            }
            Ok(())
        };
        match &self.api.pagination {
            PaginationConfig::SingleResponse { complete_pointer } => {
                if complete_pointer.is_empty() {
                    bail!("api.pagination.complete_pointer must be non-empty for single_response mode");
                }
                validate_absolute_ptr(complete_pointer, "complete_pointer")?;
            }
            PaginationConfig::NextPage {
                has_more_pointer,
                next_page_pointer,
                request_page_key,
            } => {
                if has_more_pointer.is_empty() {
                    bail!("api.pagination.has_more_pointer must be non-empty for next_page mode");
                }
                validate_absolute_ptr(has_more_pointer, "has_more_pointer")?;
                if next_page_pointer.is_empty() {
                    bail!("api.pagination.next_page_pointer must be non-empty for next_page mode");
                }
                validate_absolute_ptr(next_page_pointer, "next_page_pointer")?;
                if request_page_key.is_empty() {
                    bail!("api.pagination.request_page_key must be non-empty for next_page mode");
                }
            }
        }
        Ok(())
    }
    /// Convert to an AlphaApiClientConfig, wiring all fields including cap_markers.
    pub fn to_client_config(&self) -> crate::alpha_api::AlphaApiClientConfig {
        let auth = &self.authorization;
        let api = &self.api;
        crate::alpha_api::AlphaApiClientConfig {
            base_url: api.base_url.clone(),
            rankings_path: api.rankings_path.clone(),
            nav_info_path: api.nav_info_path.clone(),
            timeout_seconds: api.timeout_seconds,
            max_retries: api.max_retries,
            pagination: api.pagination.clone(),
            allowed_routes: auth.allowed_routes.clone(),
            allowed_fields: auth.allowed_fields.clone(),
            max_concurrent_requests: auth.max_concurrent_requests,
            min_delay_ms: auth.min_delay_ms,
            max_retry_delay_ms: auth.max_retry_delay_ms,
            cap_markers: api.cap_markers.clone(),
        }
    }
}
