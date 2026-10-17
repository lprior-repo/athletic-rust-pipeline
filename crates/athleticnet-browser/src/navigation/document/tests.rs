use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn request(id: &str, frame: &str, kind: &str) -> TestResult<EventRequestWillBeSent> {
    Ok(serde_json::from_value(serde_json::json!({
        "requestId": id,
        "loaderId": "loader",
        "documentURL": "https://example.com/page",
        "request": {
            "url": "https://example.com/page",
            "method": "GET",
            "headers": {},
            "initialPriority": "High",
            "referrerPolicy": "no-referrer"
        },
        "timestamp": 1.0,
        "wallTime": 1.0,
        "initiator": {"type": "other"},
        "redirectHasExtraInfo": false,
        "type": kind,
        "frameId": frame
    }))?)
}

fn response(id: &str, status: u16, marker: &str) -> TestResult<Arc<EventResponseReceived>> {
    Ok(Arc::new(serde_json::from_value(serde_json::json!({
        "requestId": id,
        "loaderId": "loader",
        "timestamp": 2.0,
        "type": "Document",
        "response": {
            "url": "https://example.com/page",
            "status": status,
            "statusText": "response",
            "headers": {"x-selected": marker},
            "mimeType": "text/html",
            "charset": "utf-8",
            "connectionReused": false,
            "connectionId": 1.0,
            "encodedDataLength": 128.0,
            "securityState": "unknown"
        },
        "hasExtraInfo": false
    }))?))
}

fn finished(id: &str, encoded: f64) -> TestResult<EventLoadingFinished> {
    let mut event: EventLoadingFinished = serde_json::from_value(serde_json::json!({
        "requestId": id,
        "timestamp": 3.0,
        "encodedDataLength": 0.0
    }))?;
    event.encoded_data_length = encoded;
    Ok(event)
}

#[test]
fn interleaved_prebinding_events_retain_only_the_selected_document() -> TestResult {
    let mut state = DocumentState::new("https://example.com/page".to_owned());
    let gate = ProfileGate::new();
    state.on_response(response("selected", 200, "selected")?, &gate)?;
    assert_eq!(
        state.accept_finished(&finished("unrelated", 2048.0)?)?,
        None
    );
    state.on_response(response("unrelated", 403, "unrelated")?, &gate)?;
    assert_eq!(state.accept_finished(&finished("selected", 1024.0)?)?, None);
    let selected = request("selected", "main", "Document")?;
    assert_eq!(
        state.accept_main_request(&selected, &FrameId::new("main"))?,
        Some(RequestId::new("selected"))
    );
    state.mark_navigated();
    assert!(!state.complete());
    let observation = state.into_observation();
    assert_eq!(observation.url, "https://example.com/page");
    assert_eq!(observation.status, Some(200));
    assert_eq!(
        observation
            .headers
            .get("x-selected")
            .and_then(|value| value.to_str().ok()),
        Some("selected")
    );
    assert!(!observation.body_complete);
    Ok(())
}

#[test]
fn iframe_and_non_document_requests_cannot_select_pending_metadata() -> TestResult {
    for (frame, kind) in [("iframe", "Document"), ("main", "Script")] {
        let mut state = DocumentState::new("https://example.com/page".to_owned());
        state.on_response(response("other", 403, "other")?, &ProfileGate::new())?;
        assert_eq!(state.accept_finished(&finished("other", 512.0)?)?, None);
        assert_eq!(
            state.accept_main_request(&request("other", frame, kind)?, &FrameId::new("main"))?,
            None
        );
        let observation = state.into_observation();
        assert_eq!(observation.status, None);
        assert!(!observation.body_complete);
    }
    Ok(())
}

#[test]
fn finish_before_request_preserves_later_response_without_claiming_a_body() -> TestResult {
    let mut state = DocumentState::new("https://example.com/page".to_owned());
    assert_eq!(state.accept_finished(&finished("early", 1024.0)?)?, None);
    assert_eq!(
        state.accept_main_request(
            &request("early", "main", "Document")?,
            &FrameId::new("main")
        )?,
        Some(RequestId::new("early"))
    );
    state.on_response(response("early", 200, "early")?, &ProfileGate::new())?;
    state.mark_navigated();
    assert!(!state.complete());
    let observation = state.into_observation();
    assert_eq!(observation.status, Some(200));
    assert!(!observation.body_complete);
    Ok(())
}

#[test]
fn invalid_selected_completion_cannot_authorize_body_capture() -> TestResult {
    for (total, expected) in [
        (f64::NAN, BrowserError::Protocol),
        (33_554_433.0, BrowserError::PayloadLimit),
    ] {
        let mut state = DocumentState::new("https://example.com/page".to_owned());
        assert_eq!(
            state.accept_main_request(
                &request("selected", "main", "Document")?,
                &FrameId::new("main")
            )?,
            None
        );
        assert_eq!(
            state.accept_finished(&finished("selected", total)?),
            Err(expected)
        );
        assert!(!state.complete());
        assert!(!state.into_observation().body_complete);
    }
    Ok(())
}
