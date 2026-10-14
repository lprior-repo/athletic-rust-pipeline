use super::observe_response_event;
use crate::gate::ProfileGate;
use crate::protocol::RankingsCapture;
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;

const ORIGIN: &str = "https://www.athletic.net";
const RESULTS_URL: &str = "https://www.athletic.net/api/v1/tfRankings/GetRankings";
const NAVIGATION_URL: &str = "https://www.athletic.net/api/v1/tfRankings/GetNavInfo";

fn response_event(
    url: &str,
    status: i64,
    headers: serde_json::Value,
) -> Result<EventResponseReceived, Box<dyn std::error::Error>> {
    let event: EventResponseReceived = serde_json::from_value(serde_json::json!({
        "requestId": "req-1",
        "loaderId": "loader-1",
        "timestamp": 1.0,
        "type": "XHR",
        "response": {
            "url": url,
            "status": status,
            "statusText": "response",
            "headers": headers,
            "mimeType": "application/json",
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

#[test]
fn a_bare_403_rankings_response_leaves_the_gate_open() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(RESULTS_URL, 403, serde_json::json!({}))?;
    observe_response_event(&event, RankingsCapture::Results, ORIGIN, &gate);
    check!(
        gate.is_ready(),
        "a bare 403 on the rankings route is not a human challenge"
    );
    Ok(())
}

#[test]
fn a_bare_429_rankings_response_leaves_the_gate_open() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(NAVIGATION_URL, 429, serde_json::json!({}))?;
    observe_response_event(&event, RankingsCapture::Navigation, ORIGIN, &gate);
    check!(
        gate.is_ready(),
        "a bare 429 on the rankings route is not a human challenge"
    );
    Ok(())
}

#[test]
fn an_explicit_challenge_header_revokes_the_gate() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(
        RESULTS_URL,
        403,
        serde_json::json!({"cf-mitigated": "challenge"}),
    )?;
    observe_response_event(&event, RankingsCapture::Results, ORIGIN, &gate);
    check!(
        !gate.is_ready(),
        "an explicit cf-mitigated challenge header revokes the gate"
    );
    Ok(())
}

#[test]
fn a_foreign_origin_response_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(
        "https://other.example/api/v1/tfRankings/GetRankings",
        403,
        serde_json::json!({"cf-mitigated": "challenge"}),
    )?;
    observe_response_event(&event, RankingsCapture::Results, ORIGIN, &gate);
    check!(
        gate.is_ready(),
        "a response outside the target origin never touches the gate"
    );
    Ok(())
}

#[test]
fn an_unexpected_path_response_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(
        "https://www.athletic.net/api/v1/tfRankings/Other",
        403,
        serde_json::json!({"cf-mitigated": "challenge"}),
    )?;
    observe_response_event(&event, RankingsCapture::Results, ORIGIN, &gate);
    check!(
        gate.is_ready(),
        "a response outside the rankings path never touches the gate"
    );
    Ok(())
}

#[test]
fn an_unrepresentable_status_never_revokes_the_gate() -> Result<(), Box<dyn std::error::Error>> {
    let gate = open_gate()?;
    let event = response_event(RESULTS_URL, -1, serde_json::json!({}))?;
    observe_response_event(&event, RankingsCapture::Results, ORIGIN, &gate);
    check!(
        gate.is_ready(),
        "a wire status outside u16 is protocol noise, not a challenge"
    );
    Ok(())
}
