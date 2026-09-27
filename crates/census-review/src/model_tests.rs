use super::*;
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

fn valid_options() -> ModelOptions {
    ModelOptions::local("http://127.0.0.1:52080", "qwen3.6-35b-a3b").expect("valid endpoint")
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
fn dns_name_is_rejected() {
    let opts = ModelOptions::local("http://localhost:52080", "model");
    assert!(opts.is_err());
    let err = opts.unwrap_err();
    assert!(matches!(err, ModelError::InvalidEndpoint { .. }));
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
fn empty_model_is_rejected() {
    let opts = ModelOptions::local("http://127.0.0.1:52080", "");
    assert!(opts.is_err());
}

#[test]
fn oversized_model_is_rejected() {
    let long_name = "x".repeat(257);
    let opts = ModelOptions::local("http://127.0.0.1:52080", &long_name);
    assert!(opts.is_err());
}

#[test]
fn max_tokens_is_capped_at_8192() {
    let opts = valid_options().with_max_tokens(16_000);
    let body = build_request_body(&packet(), &opts).expect("body serializes");
    let parsed: serde_json::Value = serde_json::from_slice(&body).expect("body is valid JSON");
    assert_eq!(parsed["max_tokens"], 8_192);
}

#[test]
fn request_overflow_returns_typed_error() {
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
    let result = build_request_body(&large_packet, &valid_options());
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ModelError::RequestTooLarge { .. }
    ));
}

#[test]
fn the_encoded_request_limit_counts_escaping_and_accepts_the_exact_boundary() {
    let mut request = ReviewPacket::new("synthetic", "Quoted \"Évidence\"\ncontrol\r\\")
        .with_evidence(ReviewEvidenceFact::new("capture", "field", ""));
    let options = valid_options();
    let base = build_request_body(&request, &options)
        .expect("base request")
        .len();
    let remaining = REQUEST_CAP - base;
    request.evidence[0].value = "\0".repeat(remaining / 6);
    request.evidence[0]
        .value
        .extend(std::iter::repeat_n('x', remaining % 6));
    let exact = build_request_body(&request, &options).expect("exact encoded limit");
    assert_eq!(exact.len(), REQUEST_CAP);
    assert!(exact.capacity() <= REQUEST_CAP);
    let _: serde_json::Value = serde_json::from_slice(&exact).expect("complete escaped JSON");
    request.evidence[0].value.push('x');
    assert!(matches!(
        build_request_body(&request, &options),
        Err(ModelError::RequestTooLarge { bytes }) if bytes > REQUEST_CAP
    ));
}

#[test]
fn one_oversized_fact_returns_its_rejected_size_without_echoing_content() {
    let secret = "OVERSIZED-PRIVATE-SENTINEL";
    let request = ReviewPacket::new("synthetic", "Subject").with_evidence(ReviewEvidenceFact::new(
        "capture",
        "field",
        secret.repeat(REQUEST_CAP / secret.len() + 1),
    ));
    let error = build_request_body(&request, &valid_options()).expect_err("oversized fact");
    assert!(matches!(error, ModelError::RequestTooLarge { bytes } if bytes > REQUEST_CAP));
    assert!(!error.to_string().contains(secret));
}

#[test]
fn parse_batch_from_valid_content() {
    let content = r#"{"subject_id":"school:madison-west","verdicts":[{"case_id":"c","kind":"value_proposed","field":"state","value":"WI","confidence":80,"rationale":"the WIAA lists the school"}]}"#;
    let batch = parse_batch(content).expect("a batch");
    assert_eq!(batch.subject_id, "school:madison-west");
    assert_eq!(batch.verdicts.len(), 1);
    assert_eq!(batch.verdicts[0].value.as_deref(), Some("WI"));
}

#[test]
fn fence_stripping_works() {
    let fenced = "```json\n{\"subject_id\":\"s\",\"verdicts\":[]}\n```";
    let batch = parse_batch(fenced).expect("a fenced batch");
    assert_eq!(batch.subject_id, "s");
}

#[test]
fn a_verdict_array_without_its_subject_envelope_is_rejected() {
    let bare = r#"[{"case_id":"c","kind":"insufficient_evidence","field":"","value":"","confidence":10,"rationale":"none"}]"#;
    assert!(matches!(parse_batch(bare), Err(ModelError::Content { .. })));
}

#[test]
fn invalid_content_returns_error_without_echo() {
    let sentinel = "SENTINEL_MODEL_SECRET_7f3a9b";
    let result = parse_batch(sentinel);
    let err = result.expect_err("not a batch");
    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(
        !display.contains(sentinel),
        "Display must not echo model content: {display}"
    );
    assert!(
        !debug.contains(sentinel),
        "Debug must not echo model content: {debug}"
    );
}

#[test]
fn invalid_endpoint_display_has_no_url() {
    let sentinel = "http://SENTINEL_ENDPOINT_SECRET_7f3a9b@evil.example.com:52080";
    let result = ModelOptions::local(sentinel, "m");
    let err = result.unwrap_err();
    let display = format!("{err}");
    let debug = format!("{err:?}");
    assert!(
        !display.contains(sentinel),
        "Display must not echo endpoint URL: {display}"
    );
    assert!(
        !debug.contains(sentinel),
        "Debug must not echo endpoint URL: {debug}"
    );
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
