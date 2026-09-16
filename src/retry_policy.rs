use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderMap, RETRY_AFTER},
    StatusCode,
};
use std::time::{Duration, SystemTime};

pub fn transient_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Never retry before a server deadline. Long deadlines require a later resume.
pub fn retry_after(headers: &HeaderMap, now: SystemTime) -> Result<Duration> {
    let Some(value) = headers.get(RETRY_AFTER) else {
        return Ok(Duration::ZERO);
    };
    let value = value.to_str().context("invalid Retry-After encoding")?;
    let delay = match value.parse::<u64>() {
        Ok(seconds) => Duration::from_secs(seconds),
        Err(_) => httpdate::parse_http_date(value)
            .context("invalid Retry-After date")?
            .duration_since(now)
            .map_or(Duration::ZERO, |delay| delay),
    };
    if delay > Duration::from_secs(60) {
        anyhow::bail!("Retry-After exceeds 60 second automatic retry budget; resume later");
    }
    Ok(delay)
}

pub fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(500)
        .saturating_mul(
            1_u32
                .checked_shl(attempt.saturating_sub(1).min(7))
                .map_or(128, |value| value),
        )
        .min(Duration::from_secs(60))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;
    #[test]
    fn server_deadlines_are_obeyed_or_explicitly_rejected() -> Result<()> {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("7"));
        assert_eq!(retry_after(&headers, now)?, Duration::from_secs(7));
        headers.insert(
            RETRY_AFTER,
            HeaderValue::from_str(&httpdate::fmt_http_date(now + Duration::from_secs(11)))?,
        );
        assert_eq!(retry_after(&headers, now)?, Duration::from_secs(11));
        headers.insert(RETRY_AFTER, HeaderValue::from_static("61"));
        assert!(retry_after(&headers, now).is_err());
        headers.insert(RETRY_AFTER, HeaderValue::from_static("invalid"));
        assert!(retry_after(&headers, now).is_err());
        Ok(())
    }
    #[test]
    fn permanent_client_errors_do_not_retry() {
        assert!(!transient_status(StatusCode::FORBIDDEN));
        assert!(!transient_status(StatusCode::BAD_REQUEST));
        assert!(transient_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(transient_status(StatusCode::SERVICE_UNAVAILABLE));
        assert_eq!(backoff(1), Duration::from_millis(500));
        assert_eq!(backoff(2), Duration::from_secs(1));
        assert_eq!(backoff(u32::MAX), Duration::from_secs(60));
    }
}
