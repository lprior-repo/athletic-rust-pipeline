use super::*;
use crate::consensus::tests::support::TestResult;
use census_domain::model::{ReviewCaseFact, ReviewEvidenceFact, ReviewVerdict, ReviewVerdictKind};

fn packet() -> ReviewPacket {
    ReviewPacket::new("school:madison-west", "Madison West High School")
        .with_case(ReviewCaseFact {
            case_id: "School jurisdiction unresolved:school:madison-west".to_string(),
            family: "School jurisdiction unresolved".to_string(),
            detail: "no state from any source".to_string(),
        })
        .with_evidence(ReviewEvidenceFact::new("wiaa_school", "city", "Madison"))
}

fn valid_options() -> Result<ModelOptions, ModelError> {
    ModelOptions::local("http://127.0.0.1:52080", "qwen3.6-35b-a3b")
}

#[test]
fn ipv4_loopback_is_accepted() {
    let opts = ModelOptions::local("http://127.0.0.1:52080", "model");
    assert!(opts.is_ok());
}

#[test]
fn ipv6_loopback_is_accepted() {
    let opts = ModelOptions::local("http://[::1]:52080", "model");
    assert!(opts.is_ok());
}

#[test]
fn dns_name_is_rejected() -> TestResult {
    let opts = ModelOptions::local("http://localhost:52080", "model");
    check!(opts.is_err());
    let err = match opts {
        Err(error) => error,
        Ok(_) => return Err("DNS endpoint accepted".into()),
    };
    check!(matches!(err, ModelError::InvalidEndpoint { .. }));
    Ok(())
}

#[test]
fn dns_domain_is_rejected() {
    let opts = ModelOptions::local("http://example.com:52080", "model");
    assert!(opts.is_err());
}

#[test]
fn remote_ip_is_rejected() {
    let opts = ModelOptions::local("http://192.168.1.1:52080", "model");
    assert!(opts.is_err());
}

#[test]
fn https_is_rejected() {
    let opts = ModelOptions::local("https://127.0.0.1:52080", "model");
    assert!(opts.is_err());
}

#[test]
fn query_string_is_rejected() {
    let opts = ModelOptions::local("http://127.0.0.1:52080?foo=bar", "model");
    assert!(opts.is_err());
}

#[test]
fn fragment_is_rejected() {
    let opts = ModelOptions::local("http://127.0.0.1:52080#section", "model");
    assert!(opts.is_err());
}

#[test]
fn non_root_path_is_rejected() {
    let opts = ModelOptions::local("http://127.0.0.1:52080/v1", "model");
    assert!(opts.is_err());
}

#[test]
fn userinfo_is_rejected() {
    let opts = ModelOptions::local("http://user:pass@127.0.0.1:52080", "model");
    assert!(opts.is_err());
    let password_only = ModelOptions::local("http://:pass@127.0.0.1:52080", "model");
    assert!(matches!(
        password_only,
        Err(ModelError::InvalidEndpoint { .. })
    ));
}

#[test]
fn empty_and_whitespace_only_models_are_rejected_at_the_options_boundary() {
    for model in ["", " ", "\t\r\n", "\u{2003}"] {
        assert!(matches!(
            ModelOptions::local("http://127.0.0.1:52080", model),
            Err(ModelError::InvalidEndpoint { .. })
        ));
    }
}

#[test]
fn oversized_model_is_rejected() {
    let long_name = "x".repeat(257);
    let opts = ModelOptions::local("http://127.0.0.1:52080", &long_name);
    assert!(opts.is_err());
}

#[test]
fn max_tokens_is_capped_at_8192() -> TestResult {
    let opts = valid_options()?.with_max_tokens(16_000);
    let body = build_request_body(&packet(), &opts)?;
    let parsed: serde_json::Value = serde_json::from_slice(&body)?;
    check!(eq; parsed["max_tokens"], 8_192);
    Ok(())
}

#[test]
fn request_overflow_returns_typed_error() -> TestResult {
    let large_packet = {
        let mut p = ReviewPacket::new("subj", "Subject");
        let facts: Vec<ReviewEvidenceFact> = (0..50_000)
            .map(|i| ReviewEvidenceFact::new("src", "field", format!("value-{i}")))
            .collect();
        for f in facts {
            p = p.with_evidence(f);
        }
        p
    };
    let result = build_request_body(&large_packet, &valid_options()?);
    check!(result.is_err());
    check!(matches!(result, Err(ModelError::RequestTooLarge { .. })));
    Ok(())
}

#[test]
fn the_encoded_request_limit_counts_escaping_and_accepts_the_exact_boundary() -> TestResult {
    let mut request = ReviewPacket::new("synthetic", "Quoted \"Évidence\"\ncontrol\r\\")
        .with_evidence(ReviewEvidenceFact::new("capture", "field", ""));
    let options = valid_options()?;
    let base = build_request_body(&request, &options)?.len();
    let remaining = REQUEST_CAP - base;
    request.evidence[0].value = "\0".repeat(remaining / 6);
    request.evidence[0]
        .value
        .extend(std::iter::repeat_n('x', remaining % 6));
    let exact = build_request_body(&request, &options)?;
    check!(eq; exact.len(), REQUEST_CAP);
    check!(exact.capacity() <= REQUEST_CAP);
    let _: serde_json::Value = serde_json::from_slice(&exact)?;
    request.evidence[0].value.push('x');
    check!(
        matches!(build_request_body(&request, &options), Err(ModelError::RequestTooLarge { bytes }) if bytes > REQUEST_CAP)
    );
    Ok(())
}

#[test]
fn one_oversized_fact_returns_its_rejected_size_without_echoing_content() -> TestResult {
    let secret = "OVERSIZED-PRIVATE-SENTINEL";
    let request = ReviewPacket::new("synthetic", "Subject").with_evidence(ReviewEvidenceFact::new(
        "capture",
        "field",
        secret.repeat(REQUEST_CAP / secret.len() + 1),
    ));
    let error = match build_request_body(&request, &valid_options()?) {
        Err(error) => error,
        Ok(_) => return Err("oversized fact accepted".into()),
    };
    check!(matches!(error, ModelError::RequestTooLarge { bytes } if bytes > REQUEST_CAP));
    check!(!error.to_string().contains(secret));
    Ok(())
}

#[test]
fn parse_batch_from_valid_content() -> TestResult {
    let content = r#"{"subject_id":"school:madison-west","verdicts":[{"case_id":"c","kind":"value_proposed","field":"state","value":"WI","confidence":80,"rationale":"the WIAA lists the school"}]}"#;
    let batch = parse_batch(content)?;
    check!(eq; batch.subject_id, "school:madison-west");
    check!(eq; batch.verdicts.len(), 1);
    check!(eq; batch.verdicts[0].value.as_deref(), Some("WI"));
    Ok(())
}

#[test]
fn a_verdict_array_without_its_subject_envelope_is_rejected() {
    let bare = r#"[{"case_id":"c","kind":"insufficient_evidence","field":"","value":"","confidence":10,"rationale":"none"}]"#;
    assert!(matches!(parse_batch(bare), Err(ModelError::Content { .. })));
}

#[test]
fn invalid_content_returns_error_without_echo() -> TestResult {
    let sentinel = "SENTINEL_MODEL_SECRET_7f3a9b";
    let result = parse_batch(sentinel);
    let err = match result {
        Err(error) => error,
        Ok(_) => return Err("not a batch".into()),
    };
    let display = format!("{err}");
    let debug = format!("{err:?}");
    check!(
        !display.contains(sentinel),
        "Display must not echo model content: {display}"
    );
    check!(
        !debug.contains(sentinel),
        "Debug must not echo model content: {debug}"
    );
    Ok(())
}

#[test]
fn invalid_endpoint_display_has_no_url() -> TestResult {
    let sentinel = "http://SENTINEL_ENDPOINT_SECRET_7f3a9b@evil.example.com:52080";
    let result = ModelOptions::local(sentinel, "m");
    let err = match result {
        Err(error) => error,
        Ok(_) => return Err("invalid endpoint accepted".into()),
    };
    let display = format!("{err}");
    let debug = format!("{err:?}");
    check!(
        !display.contains(sentinel),
        "Display must not echo endpoint URL: {display}"
    );
    check!(
        !debug.contains(sentinel),
        "Debug must not echo endpoint URL: {debug}"
    );
    Ok(())
}

#[test]
fn status_error_display_has_no_url() {
    let err = ModelError::Status { status: 503 };
    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(!display.contains("http"));
    assert!(!debug.contains("http"));
    assert!(display.contains("503"));
}

#[test]
fn empty_message_error_has_no_url() {
    let err = ModelError::Empty;
    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(!display.contains("http"));
    assert!(!debug.contains("http"));
}

#[test]
fn truncated_string_stays_on_char_boundary() {
    let long = "e".repeat(10);
    let cut = &long[..5];
    assert_eq!(cut.len(), 5);
}

#[test]
fn verdict_with_no_value_stays_a_verdict() {
    let empty = ReviewVerdict {
        case_id: "c".to_string(),
        kind: ReviewVerdictKind::ValueProposed,
        field: Some(String::new()),
        value: Some(String::new()),
        confidence: 70,
        rationale: "unsure".to_string(),
    };
    assert!(!empty.proposes_a_value());
}
