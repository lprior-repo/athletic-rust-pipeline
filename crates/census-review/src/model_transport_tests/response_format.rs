use std::time::Duration;

use census_domain::model::ReviewState;
use tokio::net::TcpListener;

use super::{serve, Reply};
use crate::consensus::tests::support::{audit, batch, client, lane, options, row, state, Fixture};
use crate::{run_lanes, ModelClient, ModelOptions, ModelResponseFormat};

async fn text_backend(contents: Vec<String>) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("fixture port");
    let address = listener.local_addr().expect("fixture address");
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            for content in contents {
                let (stream, _) = listener.accept().await.expect("fixture connection");
                serve(stream, Reply::TextOnly(content)).await;
            }
        })
        .await
        .expect("text-only fixture deadline");
    });
    (format!("http://{address}"), server)
}

fn prompt_client(endpoint: &str) -> ModelClient {
    ModelClient::new(
        ModelOptions::local(endpoint, "same-model-name")
            .expect("local endpoint")
            .with_response_format(ModelResponseFormat::PromptJson)
            .with_timeout(Duration::from_secs(3)),
    )
    .expect("prompt-json client")
}

#[tokio::test]
async fn text_only_backend_requires_explicit_prompt_json_before_consensus_acceptance() {
    let (fixture, case) = Fixture::school();
    let good = batch(&case, "value_proposed", "state", "WI");
    let (first, server_a) = text_backend(vec![good.clone(), good.clone()]).await;
    let (second, server_b) = lane(vec![good]);
    let constrained = [client(&first), client(&second)];
    let failed = run_lanes(&fixture.store, &constrained, &options(), "schema")
        .await
        .expect("backend rejection retained");
    assert_eq!(failed.failed, 1);
    assert_eq!(failed.accepted, 0);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
    let configured = [prompt_client(&first), client(&second)];
    let accepted = run_lanes(&fixture.store, &configured, &options(), "explicit-text")
        .await
        .expect("explicit prompt-json consensus");
    server_a.await.expect("text fixture joined");
    server_b.join().expect("schema fixture joined");
    assert_eq!(accepted.failed, 0);
    assert_eq!(accepted.accepted, 1);
    assert_eq!(row(&fixture.store).value, "WI");
    assert_eq!(state(&fixture.store), ReviewState::Resolved);
}

#[tokio::test]
async fn malformed_prompt_json_advice_remains_failed_without_extracting_embedded_json() {
    for scenario in [
        "prose",
        "trailing-prose",
        "balanced-fence",
        "unclosed-fence",
        "kind:row",
        "unknown:verdicts",
        "unknown:row",
        "confidence:101",
        "confidence:256",
        "confidence:1.5",
        "missing:subject_id",
        "missing:verdicts",
        "missing:case_id",
        "missing:kind",
        "missing:field",
        "missing:value",
        "missing:confidence",
        "missing:rationale",
        "null:subject_id",
        "null:verdicts",
        "null:case_id",
        "null:kind",
        "null:field",
        "null:value",
        "null:confidence",
        "null:rationale",
    ] {
        let (fixture, case) = Fixture::school();
        let good = batch(&case, "value_proposed", "state", "WI");
        let malformed = malformed_advice(&good, scenario);
        let (first, server_a) = text_backend(vec![malformed]).await;
        let (second, server_b) = lane(vec![good]);
        let report = run_lanes(
            &fixture.store,
            &[prompt_client(&first), client(&second)],
            &options(),
            "malformed-text",
        )
        .await
        .expect("malformed advice retained");
        server_a.await.expect("text fixture joined");
        server_b.join().expect("schema fixture joined");
        assert_eq!(report.failed, 1, "{scenario}");
        assert_eq!(report.accepted, 0, "{scenario}");
        assert_eq!(
            audit(&fixture.store)["outcome"],
            "lane_failed",
            "{scenario}"
        );
        assert_eq!(
            audit(&fixture.store)["lanes"][0]["status"],
            "failed",
            "{scenario}"
        );
        assert!(!row(&fixture.store).accepted, "{scenario}");
        assert_eq!(state(&fixture.store), ReviewState::Retained, "{scenario}");
    }
}

#[tokio::test]
async fn matching_prompt_json_proposals_cannot_introduce_an_invalid_jurisdiction() {
    let (fixture, case) = Fixture::school();
    let unsupported = batch(&case, "value_proposed", "state", "Atlantis");
    let (first, server_a) = text_backend(vec![unsupported.clone()]).await;
    let (second, server_b) = text_backend(vec![unsupported]).await;
    let report = run_lanes(
        &fixture.store,
        &[prompt_client(&first), prompt_client(&second)],
        &options(),
        "unsupported-text",
    )
    .await
    .expect("unsupported advice retained");
    server_a.await.expect("first text fixture joined");
    server_b.await.expect("second text fixture joined");
    assert_eq!(report.failed, 0);
    assert_eq!(report.accepted, 0);
    assert_eq!(report.rejected, 1);
    assert_eq!(audit(&fixture.store)["outcome"], "refused");
    assert!(!row(&fixture.store).accepted);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
}

fn malformed_advice(good: &str, scenario: &str) -> String {
    match scenario {
        "prose" => return format!("The answer is {good}"),
        "trailing-prose" => return format!("{good}\nI inferred the missing facts."),
        "balanced-fence" => return format!("```json\n{good}\n```"),
        "unclosed-fence" => return format!("```json\n{good}"),
        _ => {}
    }
    let mut value: serde_json::Value = serde_json::from_str(good).expect("valid fixture");
    let (operation, field) = scenario.split_once(':').expect("malformation selector");
    let target = if matches!(field, "subject_id" | "verdicts") {
        &mut value
    } else {
        &mut value["verdicts"][0]
    };
    match operation {
        "missing" => {
            target.as_object_mut().expect("wire object").remove(field);
        }
        "null" => target[field] = serde_json::Value::Null,
        "unknown" => target["invented"] = serde_json::json!("hallucinated fact"),
        "kind" => target["kind"] = serde_json::json!("invented_verdict"),
        "confidence" => {
            target["confidence"] = serde_json::from_str(field).expect("numeric fixture")
        }
        other => panic!("unknown malformation {other}"),
    }
    value.to_string()
}

#[tokio::test]
async fn valid_prompt_json_insufficient_evidence_remains_answered_but_unaccepted() {
    let (fixture, case) = Fixture::school();
    let insufficient = batch(&case, "insufficient_evidence", "", "");
    let (first, server_a) = text_backend(vec![insufficient.clone()]).await;
    let (second, server_b) = text_backend(vec![insufficient]).await;
    let report = run_lanes(
        &fixture.store,
        &[prompt_client(&first), prompt_client(&second)],
        &options(),
        "insufficient-text",
    )
    .await
    .expect("valid insufficient advice retained");
    server_a.await.expect("first text fixture joined");
    server_b.await.expect("second text fixture joined");
    assert_eq!(report.failed, 0);
    assert_eq!(report.accepted, 0);
    assert_eq!(report.insufficient, 1);
    assert_eq!(audit(&fixture.store)["outcome"], "insufficient_evidence");
    assert_eq!(audit(&fixture.store)["lanes"][0]["status"], "answered");
    assert_eq!(audit(&fixture.store)["lanes"][1]["status"], "answered");
    assert!(!row(&fixture.store).accepted);
    assert_eq!(state(&fixture.store), ReviewState::Retained);
}
