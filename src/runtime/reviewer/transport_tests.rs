//! Failure-classification tests for the reviewer transport.

use super::*;
use crate::runtime::source::retry::retry_after;
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
use std::time::SystemTime;

fn receipt(status: u16) -> crate::runtime::protocol::DocumentReceipt {
    crate::runtime::protocol::DocumentReceipt {
        digest: crate::domain::identity::EvidenceDigest::parse(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .expect("synthetic digest"),
        source_url: "http://127.0.0.1:9000/v1/chat/completions".to_owned(),
        http_status: status,
        media_type: "application/json".to_owned(),
        bytes: 1,
        fetched_at_unix_ms: 1,
        elapsed_ms: 1,
        rankings: None,
    }
}

#[test]
fn shared_retry_after_parser_accepts_delta_and_http_date_but_rejects_excessive_values() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("7"));
    assert_eq!(retry_after(&headers, now), Ok(Duration::from_secs(7)));
    headers.insert(RETRY_AFTER, HeaderValue::from_static("86401"));
    assert!(retry_after(&headers, now).is_err());
    let date = httpdate::fmt_http_date(now + Duration::from_secs(11));
    headers.insert(RETRY_AFTER, HeaderValue::from_str(&date).expect("date"));
    assert_eq!(retry_after(&headers, now), Ok(Duration::from_secs(11)));
}

#[test]
fn unsafe_retry_after_fails_closed_without_retrying() {
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, HeaderValue::from_static("not-a-delay"));
    let parsed = retry_after(&headers, SystemTime::UNIX_EPOCH);
    let attempt = http_failure(StatusCode::TOO_MANY_REQUESTS, receipt(429), parsed);
    assert!(!attempt.retryable());
    assert!(matches!(
        attempt,
        Attempt::Failure {
            code: FailureCode::RateLimited,
            retryable: false,
            ..
        }
    ));
}

#[test]
fn retry_after_beyond_sdk_delay_is_nonretryable_with_cooldown() {
    let attempt = http_failure(
        StatusCode::SERVICE_UNAVAILABLE,
        receipt(503),
        Ok(Duration::from_secs(2)),
    );
    assert!(!attempt.retryable());
    assert!(matches!(
        attempt,
        Attempt::Failure {
            retry_after_ms: 2_000,
            ..
        }
    ));
}

#[test]
fn missing_retry_after_allows_sdk_retry() {
    let attempt = http_failure(
        StatusCode::SERVICE_UNAVAILABLE,
        receipt(503),
        Ok(Duration::ZERO),
    );
    assert!(attempt.retryable());
    assert!(matches!(
        attempt,
        Attempt::Failure {
            retry_after_ms: 0,
            ..
        }
    ));
}
