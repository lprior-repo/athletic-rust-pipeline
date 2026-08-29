/// Errors returned by the alpha API client.
#[derive(Debug, thiserror::Error)]
pub enum AlphaApiError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("timeout after {} ms", milliseconds)]
    Timeout { milliseconds: u64 },
    #[error("unauthorised: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("rate limited (429): no Retry-After header present or invalid")]
    RateLimitedNoRetryAfter,
    #[error("rate limited (429): Retry-After exceeded max retry count ({}) after {} ms", max_retries, total_delay_ms)]
    RateLimitedExhausted { max_retries: usize, total_delay_ms: u64 },
    #[error("server error {} after {} retries", status, retries)]
    ServerErrorExhausted { status: u16, retries: usize },
    #[error("unexpected status {}: {}", status, body)]
    UnexpectedStatus { status: u16, body: String },
    #[error("incomplete response: {0}")]
    Incomplete(String),
    #[error("missing required JSON pointer value at {0}")]
    MissingPointer(String),
    #[error("response truncated or capped without continuation metadata")]
    TruncatedWithoutContinuation,
    #[error("invalid concurrency: must be exactly 1")]
    InvalidConcurrency,
    #[error("authorization disabled or permission_reference empty")]
    AuthorizationDisabled,
    #[error("response body too large (exceeded {} bytes)", limit)]
    BodyTooLarge { limit: u64 },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Configuration extracted from `AlphaConfig` for constructing the client.
pub struct AlphaApiClientConfig {
    pub base_url: String,
    pub rankings_path: String,
    pub nav_info_path: String,
    pub timeout_seconds: u64,
    pub max_retries: usize,
    pub pagination: crate::alpha_model::PaginationConfig,
    pub allowed_routes: Vec<String>,
    pub allowed_fields: Vec<String>,
    pub max_concurrent_requests: usize,
    pub max_body_bytes: u64,
    pub cap_markers: Vec<String>,
    pub auth_enabled: bool,
    pub permission_reference: String,
    pub min_delay_ms: u64,
    pub max_retry_delay_ms: u64,
}
/// Strict RFC6901 JSON pointer validation: non-empty, starts with '/',
/// every '~' followed by '0' or '1'.
fn validate_absolute_pointer(ptr: &str, name: &str) -> Result<(), AlphaApiError> {
    if ptr.is_empty() {
        return Err(AlphaApiError::InvalidConfig(format!(
            "{name} must be non-empty"
        )));
    }
    if !ptr.starts_with('/') {
        return Err(AlphaApiError::InvalidConfig(format!(
            "{name} must start with '/' (got '{ptr}')"
        )));
    }
    for (pos, ch) in ptr.chars().enumerate() {
        if ch == '~' {
            match ptr.chars().nth(pos + 1) {
                Some('0') | Some('1') => {}
                Some(other) => {
                    return Err(AlphaApiError::InvalidConfig(format!(
                        "{name} has invalid RFC6901 escape '~{other}' at position {pos}",
                    )));
                }
                None => {
                    return Err(AlphaApiError::InvalidConfig(format!(
                        "{name} has trailing '~' at position {pos}",
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Validate a PaginationConfig: all pointer fields must be
/// non-empty strict RFC6901 pointers; request_page_key must be non-empty.
pub fn validate_pagination_config(
    pagination: &crate::alpha_model::PaginationConfig,
) -> Result<(), AlphaApiError> {
    match pagination {
        crate::alpha_model::PaginationConfig::SingleResponse {
            complete_pointer,
        } => {
            validate_absolute_pointer(complete_pointer, "complete_pointer")?;
        }
        crate::alpha_model::PaginationConfig::NextPage {
            has_more_pointer,
            next_page_pointer,
            request_page_key,
        } => {
            validate_absolute_pointer(has_more_pointer, "has_more_pointer")?;
            validate_absolute_pointer(next_page_pointer, "next_page_pointer")?;
            if request_page_key.is_empty() {
                return Err(AlphaApiError::InvalidConfig(
                    "request_page_key must be non-empty".into(),
                ));
            }
        }
    }
    Ok(())
}

