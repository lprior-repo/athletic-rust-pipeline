//! What the transport still owns about retrying: the classification, not the retry.
//!
//! ADR-002 gives retrying one owner - the invocation retry declared on the handler - so the
//! transport performs a single attempt and reports what it saw. Nothing here asks the source
//! again: no attempt budget, no delay ladder, no sleep inside the invocation. What survives is the
//! part a caller needs to judge the attempt itself: what a `Retry-After` header asked for, which is
//! what the 429 cooldown is built from.
//!
//! The pipeline crate keeps `retryable_status` - the status classification its HTTP lanes share -
//! and imports the header arithmetic from here, because a browser cooldown and an HTTP lane read
//! the same header the same way.

use crate::clock::{self, Clock};
use httpdate::parse_http_date;
use reqwest::header::{HeaderMap, RETRY_AFTER};
use std::time::{Duration, SystemTime};

/// Bound on the `Retry-After` a source may ask for. An instant beyond it is not a delay a run can
/// honour, so it fails closed instead of parking the lane on an unusable instant.
pub const MAX_RETRY_DELAY: Duration = Duration::from_secs(86_400);

/// Parse `Retry-After` against the injected wall clock.
///
/// `Retry-After` is an absolute HTTP date or a delta from *now*, so this site is wall clock by
/// definition and must not be switched to the monotonic reading. A clock that cannot be read fails
/// closed with the same `Err` an unusable header produces, which makes the caller non-retryable
/// instead of retrying on a substituted instant.
pub fn retry_after_now(clock: &dyn Clock, headers: &HeaderMap) -> Result<Duration, &'static str> {
    let unix_ms = clock
        .now_unix_ms()
        .map_err(|_| "system clock is unreadable")?;
    let now = clock::unix_ms_to_system_time(unix_ms)
        .ok_or("system clock is outside the representable range")?;
    retry_after(headers, now)
}

/// Parse `Retry-After` against a caller-supplied instant.
pub fn retry_after(headers: &HeaderMap, now: SystemTime) -> Result<Duration, &'static str> {
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
