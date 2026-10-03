#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use athleticnet_browser::clock::{Clock, ClockError};
use athleticnet_browser::{BrowserError, BrowserOutcome, BrowserResponse, Verdict};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::StatusCode;
use std::path::PathBuf;

const FIXTURE_FETCHED_AT_MS: u64 = 1_758_542_400_000;

struct FixedClock;

impl Clock for FixedClock {
    fn now_instant(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }

    fn now_unix_ms(&self) -> Result<u64, ClockError> {
        Ok(FIXTURE_FETCHED_AT_MS)
    }
}

fn fixture(name: &str) -> Result<String, std::io::Error> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/wire")
        .join(name);
    std::fs::read_to_string(&path)
}

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
        response_url: None,
        headers,
        body: br#"<html><head><title>Just a moment...</title></head><body><script src="/cdn-cgi/challenge-platform/h/b"></script></body></html>"#
            .to_vec(),
        rankings: None,
    }
}

#[test]
fn the_capture_fixture_is_what_the_transport_classifies() -> Result<(), Box<dyn std::error::Error>>
{
    let outcome = BrowserOutcome::captured(challenged_response(), &FixedClock);
    let file = fixture("athleticnet-browser-capture.json")?;

    let parsed: BrowserOutcome = serde_json::from_str(&file)?;
    check!(eq; parsed, outcome);

    let BrowserOutcome::Captured(capture) = &parsed else {
        return Err("the capture fixture decoded as a non-capture outcome".into());
    };
    check!(
        capture.challenge,
        "the body's challenge markers did not survive the wire"
    );
    check!(eq; capture.retry_after_ms, Some(30_000));
    check!(eq; capture.fetched_at_ms, Some(FIXTURE_FETCHED_AT_MS));
    check!(eq; capture
    .response
    .headers
    .get_all("set-cookie")
    .iter()
    .count(),
2,
"a repeated header must keep every value it carried");
    check!(eq; parsed.verdict(),
Verdict::HumanRequired,
"a challenged capture is a human step, not a retry");
    check!(eq; parsed.response().map(|response| response.status),
Some(StatusCode::FORBIDDEN));
    Ok(())
}

#[test]
fn the_failure_fixture_is_what_the_transport_classifies() -> Result<(), Box<dyn std::error::Error>>
{
    let outcome = BrowserOutcome::failed(BrowserError::HumanRequired);
    let file = fixture("athleticnet-browser-failure.json")?;

    let parsed: BrowserOutcome = serde_json::from_str(&file)?;
    check!(eq; parsed, outcome);
    check!(parsed.response().is_none(), "a failure carries no capture");
    check!(parsed.verdict().human_required());
    check!(
        !parsed.verdict().retryable(),
        "a human step is not something another invocation resolves"
    );
    Ok(())
}

#[test]
fn every_transport_error_carries_a_verdict() -> Result<(), Box<dyn std::error::Error>> {
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
        check!(eq; Verdict::of(error), verdict, "{error}");
        check!(eq; BrowserOutcome::failed(error).verdict(), verdict, "{error}");
    }
    Ok(())
}

#[test]
fn the_wire_rejects_what_the_reader_does_not_know() -> Result<(), Box<dyn std::error::Error>> {
    for unknown in [
        r#"{"outcome":"failed","error":"timeout","verdict":"retryable","extra":1}"#,
        r#"{"outcome":"failed","error":"gated","verdict":"terminal"}"#,
        r#"{"outcome":"failed","error":"timeout","verdict":"later"}"#,
        r#"{"outcome":"skipped"}"#,
    ] {
        check!(
            serde_json::from_str::<BrowserOutcome>(unknown).is_err(),
            "{unknown} was accepted"
        );
    }
    Ok(())
}

#[test]
fn a_header_that_is_not_text_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut response = challenged_response();
    response
        .headers
        .append("x-opaque", HeaderValue::from_bytes(b"\xff\xfe")?);
    let outcome = BrowserOutcome::captured(response, &FixedClock);
    check!(
        serde_json::to_string(&outcome).is_err(),
        "a header value that is not text must not be silently rewritten for the wire"
    );
    Ok(())
}
