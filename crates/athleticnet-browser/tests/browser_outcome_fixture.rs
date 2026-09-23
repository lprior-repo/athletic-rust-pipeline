//! The wire two crates read, pinned to committed bytes.
//!
//! `fixtures/wire/athleticnet-browser-capture.json` and `athleticnet-browser-failure.json` are
//! authored here, from this crate's own serialization, and are read byte for byte by the census
//! mirror's test. A field added, renamed, dropped, reordered or reclassified on either side fails
//! one of these assertions - which is the point: the outcome vocabulary is one contract, not two
//! similar ones.
//!
//! These tests use the public API only, because that is all a mirror can see.

use athleticnet_browser::clock::{Clock, ClockError};
use athleticnet_browser::{BrowserError, BrowserOutcome, BrowserResponse, Verdict};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::StatusCode;
use std::path::PathBuf;

/// The instant the fixtures were authored at, so `fetched_at_ms` is an exact pin.
const FIXTURE_FETCHED_AT_MS: u64 = 1_758_542_400_000;

/// A clock fixed at [`FIXTURE_FETCHED_AT_MS`]: timing the transport takes itself must be
/// reproducible for the fixture to pin anything.
struct FixedClock;

impl Clock for FixedClock {
    fn now_instant(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }

    fn now_unix_ms(&self) -> Result<u64, ClockError> {
        Ok(FIXTURE_FETCHED_AT_MS)
    }
}

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/wire")
        .join(name);
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("read {}: {error}", path.display()),
    }
}

fn pretty<T: serde::Serialize>(value: &T) -> String {
    let mut encoded = serde_json::to_string_pretty(value).expect("serialize");
    encoded.push('\n');
    encoded
}

/// The response the capture fixture pins: a challenge page, refused with `Retry-After`.
///
/// The challenge is carried by the body, not by `cf-mitigated`, so a reader that can observe
/// `"challenge": true` has proved the base64 body survived the wire intact.
fn challenged_response() -> BrowserResponse {
    let mut headers = HeaderMap::new();
    headers.append(
        "content-type",
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.append("retry-after", HeaderValue::from_static("30"));
    headers.append(
        "set-cookie",
        HeaderValue::from_static("session=abc; Path=/"),
    );
    headers.append(
        "set-cookie",
        HeaderValue::from_static("cf_clearance=def; Path=/"),
    );
    BrowserResponse {
        status: StatusCode::FORBIDDEN,
        headers,
        body: br#"<html><head><title>Just a moment...</title></head><body><script src="/cdn-cgi/challenge-platform/h/b"></script></body></html>"#
            .to_vec(),
        rankings: None,
    }
}

#[test]
fn the_capture_fixture_is_what_the_transport_classifies() {
    let outcome = BrowserOutcome::captured(challenged_response(), &FixedClock);
    let file = fixture("athleticnet-browser-capture.json");
    assert_eq!(
        pretty(&outcome),
        file,
        "the classified capture and the committed fixture have diverged"
    );

    let parsed: BrowserOutcome = match serde_json::from_str(&file) {
        Ok(parsed) => parsed,
        Err(error) => panic!("the fixture is not a BrowserOutcome: {error}"),
    };
    assert_eq!(parsed, outcome);
    assert_eq!(
        pretty(&parsed),
        file,
        "the fixture does not round-trip byte for byte"
    );

    let BrowserOutcome::Captured(capture) = &parsed else {
        panic!("the capture fixture must read as a capture");
    };
    assert!(
        capture.challenge,
        "the body's challenge markers did not survive the wire"
    );
    assert_eq!(capture.retry_after_ms, Some(30_000));
    assert_eq!(capture.fetched_at_ms, Some(FIXTURE_FETCHED_AT_MS));
    assert_eq!(
        capture
            .response
            .headers
            .get_all("set-cookie")
            .iter()
            .count(),
        2,
        "a repeated header must keep every value it carried"
    );
    assert_eq!(
        parsed.verdict(),
        Verdict::HumanRequired,
        "a challenged capture is a human step, not a retry"
    );
    assert_eq!(
        parsed.response().map(|response| response.status),
        Some(StatusCode::FORBIDDEN)
    );
}

#[test]
fn the_failure_fixture_is_what_the_transport_classifies() {
    let outcome = BrowserOutcome::failed(BrowserError::HumanRequired);
    let file = fixture("athleticnet-browser-failure.json");
    assert_eq!(
        pretty(&outcome),
        file,
        "the classified failure and the committed fixture have diverged"
    );

    let parsed: BrowserOutcome =
        serde_json::from_str(&file).expect("the fixture is a BrowserOutcome");
    assert_eq!(parsed, outcome);
    assert_eq!(
        pretty(&parsed),
        file,
        "the fixture does not round-trip byte for byte"
    );
    assert!(parsed.response().is_none(), "a failure carries no capture");
    assert!(parsed.verdict().human_required());
    assert!(
        !parsed.verdict().retryable(),
        "a human step is not something another invocation resolves"
    );
}

#[test]
fn every_transport_error_carries_a_verdict() {
    let classified = [
        (BrowserError::Transport, Verdict::Retryable),
        (BrowserError::Timeout, Verdict::Retryable),
        (BrowserError::HumanRequired, Verdict::HumanRequired),
        (BrowserError::PayloadLimit, Verdict::Terminal),
        (BrowserError::Redirect, Verdict::Terminal),
        (BrowserError::Unavailable, Verdict::Terminal),
        (BrowserError::Protocol, Verdict::Terminal),
        (BrowserError::Shutdown, Verdict::Terminal),
        (BrowserError::TaskPanicked, Verdict::Terminal),
    ];
    for (error, verdict) in classified {
        assert_eq!(Verdict::of(error), verdict, "{error}");
        assert_eq!(BrowserOutcome::failed(error).verdict(), verdict, "{error}");
    }
}

#[test]
fn the_wire_rejects_what_the_reader_does_not_know() {
    for unknown in [
        r#"{"outcome":"failed","error":"timeout","verdict":"retryable","extra":1}"#,
        r#"{"outcome":"failed","error":"gated","verdict":"terminal"}"#,
        r#"{"outcome":"failed","error":"timeout","verdict":"later"}"#,
        r#"{"outcome":"skipped"}"#,
    ] {
        assert!(
            serde_json::from_str::<BrowserOutcome>(unknown).is_err(),
            "{unknown} was accepted"
        );
    }
}

#[test]
fn a_header_that_is_not_text_fails_closed() {
    let mut response = challenged_response();
    response.headers.append(
        "x-opaque",
        HeaderValue::from_bytes(b"\xff\xfe").expect("opaque header value"),
    );
    let outcome = BrowserOutcome::captured(response, &FixedClock);
    assert!(
        serde_json::to_string(&outcome).is_err(),
        "a header value that is not text must not be silently rewritten for the wire"
    );
}
