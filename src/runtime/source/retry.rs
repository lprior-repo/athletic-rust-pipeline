use super::super::clock::{self, Clock};
use httpdate::parse_http_date;
use reqwest::header::{HeaderMap, RETRY_AFTER};
use std::time::{Duration, SystemTime};

pub(crate) const MAX_RETRY_DELAY: Duration = Duration::from_secs(86_400);
pub(crate) const MAX_ATTEMPTS: usize = 4;
const RATE_LIMIT_FALLBACK: [Duration; 3] = [
    Duration::from_secs(60),
    Duration::from_secs(120),
    Duration::from_secs(240),
];
const TRANSIENT_FALLBACK: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

pub(crate) fn next_delay(
    attempt_index: usize,
    attempt: &super::http::AttemptResult,
    source_interval: Duration,
) -> Result<Duration, &'static str> {
    if !attempt.retryable {
        return Err("non-retryable source attempt cannot be delayed");
    }
    if attempt.retry_after_ms != 0 {
        return Ok(Duration::from_millis(attempt.retry_after_ms));
    }
    let fallback = if attempt.status == Some(429) {
        RATE_LIMIT_FALLBACK
    } else {
        TRANSIENT_FALLBACK
    };
    let index = attempt_index.min(fallback.len().saturating_sub(1));
    let fallback_delay = fallback
        .get(index)
        .copied()
        .ok_or("retry fallback index exceeded bounds")?;
    Ok(source_interval.max(fallback_delay))
}

pub(crate) fn retryable_status(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

/// Parse `Retry-After` against the injected wall clock.
///
/// `Retry-After` is an absolute HTTP date or a delta from *now*, so this site is wall clock by
/// definition and must not be switched to the monotonic reading. A clock that cannot be read fails
/// closed with the same `Err` an unusable header produces, which makes the caller non-retryable
/// instead of retrying on a substituted instant.
pub(crate) fn retry_after_now(clock: &dyn Clock, headers: &HeaderMap) -> Result<Duration, &'static str> {
    let unix_ms = clock.now_unix_ms().map_err(|_| "system clock is unreadable")?;
    let now = clock::unix_ms_to_system_time(unix_ms)
        .ok_or("system clock is outside the representable range")?;
    retry_after(headers, now)
}

pub(crate) fn retry_after(headers: &HeaderMap, now: SystemTime) -> Result<Duration, &'static str> {
    let mut values = headers.get_all(RETRY_AFTER).iter();
    let Some(value) = values.next() else {
        return Ok(Duration::ZERO);
    };
    if values.next().is_some() {
        return Err("multiple Retry-After headers are ambiguous");
    }
    let value = value
        .to_str()
        .map_err(|_| "invalid Retry-After encoding")?
        .trim_matches([' ', '\t']);
    let delay = if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Duration::from_secs(
            value
                .parse::<u64>()
                .map_err(|_| "Retry-After seconds overflow")?,
        )
    } else {
        parse_http_date(value)
            .map_err(|_| "invalid Retry-After date")?
            .duration_since(now)
            .map_or(Duration::ZERO, |delay| delay)
    };
    (delay <= MAX_RETRY_DELAY)
        .then_some(delay)
        .ok_or("Retry-After exceeds bounded retry delay")
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn parses_delta_and_http_date_with_bound() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("7"));
        assert_eq!(
            retry_after(&headers, now).expect("delta"),
            Duration::from_secs(7)
        );
        headers.insert(RETRY_AFTER, HeaderValue::from_static("86401"));
        assert!(retry_after(&headers, now).is_err());
        let date = httpdate::fmt_http_date(now + Duration::from_secs(11));
        headers.insert(RETRY_AFTER, HeaderValue::from_str(&date).expect("date"));
        assert_eq!(
            retry_after(&headers, now).expect("date"),
            Duration::from_secs(11)
        );
    }

    #[test]
    fn retries_only_rate_limit_and_server_failures() {
        assert!(retryable_status(429));
        assert!(retryable_status(503));
        assert!(!retryable_status(403));
        assert!(!retryable_status(400));
    }

    #[test]
    fn absent_header_allows_retry_but_duplicate_or_signed_headers_fail_closed() {
        let now = SystemTime::UNIX_EPOCH;
        let mut headers = HeaderMap::new();
        assert_eq!(retry_after(&headers, now), Ok(Duration::ZERO));
        headers.append(RETRY_AFTER, HeaderValue::from_static("7"));
        headers.append(RETRY_AFTER, HeaderValue::from_static("7"));
        assert!(retry_after(&headers, now).is_err());
        headers.insert(RETRY_AFTER, HeaderValue::from_static("+7"));
        assert!(retry_after(&headers, now).is_err());
        headers.insert(RETRY_AFTER, HeaderValue::from_static(" \t7\t "));
        assert_eq!(retry_after(&headers, now), Ok(Duration::from_secs(7)));
    }
}
