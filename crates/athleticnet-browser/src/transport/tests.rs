use super::{record_response, BrowserError, BrowserResponse};
use crate::clock::TestClock;
use crate::gate::ProfileGate;
use crate::BrowserCapture;
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::StatusCode;

fn response_event(
    status: i64,
    headers: serde_json::Value,
) -> Result<EventResponseReceived, Box<dyn std::error::Error>> {
    let event: EventResponseReceived = serde_json::from_value(serde_json::json!({
        "requestId": "req-1",
        "loaderId": "loader-1",
        "timestamp": 1.0,
        "type": "Document",
        "response": {
            "url": "https://www.athletic.net/team/123",
            "status": status,
            "statusText": "response",
            "headers": headers,
            "mimeType": "text/plain; charset=utf-8",
            "charset": "utf-8",
            "connectionReused": false,
            "connectionId": 1.0,
            "encodedDataLength": 128.0,
            "securityState": "unknown"
        },
        "hasExtraInfo": false
    }))?;
    Ok(event)
}

fn open_gate() -> Result<ProfileGate, Box<dyn std::error::Error>> {
    let gate = ProfileGate::new();
    let generation = gate.snapshot().generation;
    check!(
        gate.try_open(generation),
        "gate opens from a clean snapshot"
    );
    Ok(gate)
}

fn plain_response(status: StatusCode, headers: HeaderMap, body: &[u8]) -> BrowserResponse {
    BrowserResponse {
        status,
        response_url: Some("https://www.athletic.net/team/123".to_owned()),
        headers,
        body: body.to_vec(),
        rankings: None,
    }
}

#[test]
fn a_bare_403_records_the_response_without_revoking_the_gate(
) -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(403, serde_json::json!({}))?;
    let mut response = None;
    let mut challenge_seen = false;
    record_response(&event, &mut response, &mut challenge_seen, &gate)?;
    check!(
        !challenge_seen,
        "a bare 403 carries no human-challenge signal"
    );
    check!(
        gate.is_ready(),
        "a bare 403 retains the response without revoking the gate"
    );
    let evidence = response.ok_or("missing response evidence")?;
    check!(eq; evidence.status, 403);
    check!(eq; evidence.response_url, "https://www.athletic.net/team/123".to_owned());
    Ok(())
}

#[test]
fn a_bare_429_records_the_response_without_revoking_the_gate(
) -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(429, serde_json::json!({}))?;
    let mut response = None;
    let mut challenge_seen = false;
    record_response(&event, &mut response, &mut challenge_seen, &gate)?;
    check!(
        !challenge_seen,
        "a bare 429 carries no human-challenge signal"
    );
    check!(
        gate.is_ready(),
        "a bare 429 retains the response without revoking the gate"
    );
    let evidence = response.ok_or("missing response evidence")?;
    check!(eq; evidence.status, 429);
    check!(eq; evidence.response_url, "https://www.athletic.net/team/123".to_owned());
    Ok(())
}

#[test]
fn an_explicit_challenge_header_revokes_the_gate() -> Result<(), Box<dyn std::error::Error>> {
    for status in [403_i64, 429] {
        let gate = open_gate()?;
        let event = response_event(status, serde_json::json!({"cf-mitigated": "challenge"}))?;
        let mut response = None;
        let mut challenge_seen = false;
        record_response(&event, &mut response, &mut challenge_seen, &gate)?;
        check!(
            challenge_seen,
            "an explicit cf-mitigated challenge header marks the challenge"
        );
        check!(
            !gate.is_ready(),
            "an explicit challenge header revokes the gate"
        );
        let evidence = response.ok_or("missing response evidence")?;
        check!(eq; evidence.status, u16::try_from(status)?);
    }
    Ok(())
}

#[test]
fn a_redirect_status_still_rejects_before_recording() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(302, serde_json::json!({}))?;
    let mut response = None;
    let mut challenge_seen = false;
    check!(matches!(
        record_response(&event, &mut response, &mut challenge_seen, &gate),
        Err(BrowserError::Redirect)
    ));
    check!(
        response.is_none(),
        "a redirect records no response evidence"
    );
    Ok(())
}

#[test]
fn a_bare_403_capture_keeps_status_body_and_no_challenge() -> Result<(), Box<dyn std::error::Error>>
{
    let clock = TestClock::at(1_700_000_000_000);
    let capture = BrowserCapture::classify(
        plain_response(StatusCode::FORBIDDEN, HeaderMap::new(), b"bare denial"),
        &clock,
    );
    check!(
        !capture.challenge,
        "a bare 403 classifies without a human challenge"
    );
    check!(eq; capture.response.status, StatusCode::FORBIDDEN);
    check!(eq; capture.response.body, b"bare denial".to_vec());
    check!(
        capture.retry_after_ms.is_none(),
        "a bare 403 without Retry-After carries no cooldown"
    );
    Ok(())
}

#[test]
fn a_bare_429_capture_keeps_retry_after_cooldown_and_no_challenge(
) -> Result<(), Box<dyn std::error::Error>> {
    let clock = TestClock::at(1_700_000_000_000);
    let mut headers = HeaderMap::new();
    headers.insert(RETRY_AFTER, reqwest::header::HeaderValue::from_static("2"));
    let capture = BrowserCapture::classify(
        plain_response(StatusCode::TOO_MANY_REQUESTS, headers, b"slow down"),
        &clock,
    );
    check!(
        !capture.challenge,
        "a bare 429 classifies without a human challenge"
    );
    check!(eq; capture.response.status, StatusCode::TOO_MANY_REQUESTS);
    check!(eq; capture.response.body, b"slow down".to_vec());
    check!(eq; capture.retry_after_ms, Some(2_000));
    Ok(())
}
