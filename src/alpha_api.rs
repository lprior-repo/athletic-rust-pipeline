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
    pub min_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub cap_markers: Vec<String>,
}

