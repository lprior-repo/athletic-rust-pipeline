use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn event(frame: Option<&str>, scripts: &[&str]) -> TestResult<EventRequestWillBeSent> {
    let frames: Vec<_> = scripts
        .iter()
        .map(|script| {
            serde_json::json!({
                "scriptId": script,
                "functionName": "fetch",
                "url": "adlaw-transport.js",
                "lineNumber": 0,
                "columnNumber": 0
            })
        })
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "requestId": "request",
        "loaderId": "loader",
        "documentURL": "https://example.com/",
        "frameId": frame,
        "request": {
            "url": "https://example.com/api",
            "method": "GET",
            "headers": {},
            "initialPriority": "High",
            "referrerPolicy": "no-referrer"
        },
        "timestamp": 1.0,
        "wallTime": 1.0,
        "redirectHasExtraInfo": false,
        "initiator": {"type": "script", "stack": {"callFrames": frames}}
    }))?)
}

#[test]
fn exact_frame_and_script_match_even_with_repeated_owned_call_frames() -> TestResult {
    let binding = RequestBinding::new(FrameId::new("main"), ScriptId::new("issued"));
    for scripts in [
        vec!["issued"],
        vec!["caller", "issued"],
        vec!["issued", "issued"],
    ] {
        assert!(binding.matches(&event(Some("main"), &scripts)?));
    }
    Ok(())
}

#[test]
fn copied_source_labels_and_function_names_cannot_replace_issued_identity() -> TestResult {
    let binding = RequestBinding::new(FrameId::new("main"), ScriptId::new("issued"));
    for (frame, scripts) in [
        (Some("foreign"), vec!["issued"]),
        (None, vec!["issued"]),
        (Some("main"), vec!["forged"]),
        (Some("main"), Vec::new()),
    ] {
        assert!(!binding.matches(&event(frame, &scripts)?));
    }
    Ok(())
}

#[test]
fn missing_stack_and_non_script_initiators_cannot_issue_a_binding() -> TestResult {
    let binding = RequestBinding::new(FrameId::new("main"), ScriptId::new("issued"));
    let mut absent = event(Some("main"), &["issued"])?;
    absent.initiator.stack = None;
    assert!(!binding.matches(&absent));
    let mut parser = event(Some("main"), &["issued"])?;
    parser.initiator.r#type = InitiatorType::Parser;
    assert!(!binding.matches(&parser));
    Ok(())
}

#[test]
fn excessive_stacks_refuse_instead_of_accepting_a_truncated_prefix() -> TestResult {
    let binding = RequestBinding::new(FrameId::new("main"), ScriptId::new("issued"));
    let at_limit = vec!["issued"; 128];
    assert!(binding.matches(&event(Some("main"), &at_limit)?));
    let over_limit = vec!["issued"; 129];
    assert!(!binding.matches(&event(Some("main"), &over_limit)?));
    Ok(())
}
