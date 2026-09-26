//! The client's two testable halves: the request it builds, and the content it accepts.

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

fn options() -> ModelOptions {
    ModelOptions::local("http://127.0.0.1:52080/", "qwen3.6-35b-a3b")
}

#[test]
fn the_endpoint_is_normalized_and_the_completions_url_is_built_from_it() {
    let options = options();
    assert_eq!(options.endpoint, "http://127.0.0.1:52080");
    assert_eq!(
        options.completions_url(),
        "http://127.0.0.1:52080/v1/chat/completions"
    );
}

#[test]
fn the_request_asks_for_a_schema_shaped_answer_without_thinking() {
    let body = build_request_body(&packet(), &options());
    assert_eq!(body["model"], "qwen3.6-35b-a3b");
    assert_eq!(body["temperature"], 0);
    assert_eq!(body["response_format"]["type"], "json_schema");
    let schema = &body["response_format"]["json_schema"]["schema"];
    assert_eq!(schema["properties"]["verdicts"]["items"]["type"], "object");
    assert_eq!(
        schema["properties"]["verdicts"]["items"]["properties"]["kind"]["enum"],
        serde_json::json!(["value_proposed", "insufficient_evidence"])
    );
    assert_eq!(
        schema["additionalProperties"], false,
        "the grammar must not admit fields the reader will ignore"
    );
    assert_eq!(body["chat_template_kwargs"]["enable_thinking"], false);
}

#[test]
fn the_user_message_carries_the_subject_its_cases_and_its_evidence() {
    let body = build_request_body(&packet(), &options());
    let user = body["messages"][1]["content"]
        .as_str()
        .expect("the user message is text");
    assert!(user.contains("School jurisdiction unresolved:school:madison-west"));
    assert!(user.contains("wiaa_school: city = Madison"));
    assert!(user.contains("no state from any source"));
    assert!(
        body["messages"][0]["content"]
            .as_str()
            .expect("the system message is text")
            .contains("Never answer for a case id you were not given"),
        "the rule that keeps a model inside the question is part of the request"
    );
}

#[test]
fn a_batch_is_read_from_the_assistant_message() {
    let body = serde_json::json!({
        "choices": [{ "message": { "content": "{\"subject_id\":\"school:madison-west\",\
            \"verdicts\":[{\"case_id\":\"School jurisdiction unresolved:school:madison-west\",\
            \"kind\":\"value_proposed\",\"field\":\"state\",\"value\":\"WI\",\
            \"confidence\":80,\"rationale\":\"the WIAA lists the school\"}]}" } }]
    });
    let content = message_content(&body).expect("content");
    let batch = parse_batch(&content).expect("a batch");
    assert_eq!(batch.subject_id, "school:madison-west");
    assert_eq!(batch.verdicts.len(), 1);
    assert_eq!(batch.verdicts[0].value.as_deref(), Some("WI"));
}

#[test]
fn an_empty_message_is_reported_as_empty_rather_than_parsed() {
    let body = serde_json::json!({ "choices": [{ "message": { "content": "   " } }] });
    assert!(message_content(&body).is_none());
}

#[test]
fn a_fenced_answer_is_read_and_a_bare_array_is_accepted_without_a_subject() {
    let fenced = "```json\n{\"subject_id\":\"school:madison-west\",\"verdicts\":[]}\n```";
    let batch = parse_batch(fenced).expect("a fenced batch");
    assert_eq!(batch.subject_id, "school:madison-west");
    assert!(batch.verdicts.is_empty());

    let bare = r#"[{"case_id":"c","kind":"insufficient_evidence","field":"","value":"","confidence":10,"rationale":"none"}]"#;
    let batch = parse_batch(bare).expect("a bare array");
    assert_eq!(batch.subject_id, "");
    assert_eq!(batch.verdicts.len(), 1);
    assert_eq!(
        batch.verdicts[0].kind,
        ReviewVerdictKind::InsufficientEvidence
    );
}

#[test]
fn an_unreadable_answer_is_an_error_that_quotes_what_arrived() {
    let error = parse_batch("I could not decide.").expect_err("not a batch");
    let message = error.to_string();
    assert!(
        message.contains("I could not decide."),
        "the operator needs the model's text to fix the prompt: {message}"
    );
}

#[test]
fn truncation_keeps_a_string_on_a_character_boundary() {
    assert_eq!(truncate("short", 400), "short");
    let long = "é".repeat(10);
    let cut = truncate(&long, 5);
    assert!(cut.starts_with('é'));
    assert!(cut.ends_with('…'));
    assert!(cut.len() <= 5 + '…'.len_utf8());
}

#[test]
fn a_verdict_with_no_value_stays_a_verdict_the_reader_can_demote() {
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
