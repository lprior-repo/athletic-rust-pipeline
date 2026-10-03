#[path = "response_format/shapes.rs"]
mod shapes;
use std::time::Duration;

use census_domain::model::ReviewState;
use tokio::net::TcpListener;

use super::{serve, Reply};
use crate::consensus::tests::support::{
    audit, batch, client, lane, options, row, state, Fixture, TestResult,
};
use crate::{run_lanes, ModelClient, ModelOptions, ModelResponseFormat};

async fn text_backend(
    contents: Vec<String>,
) -> TestResult<(String, tokio::task::JoinHandle<TestResult>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            for content in contents {
                let (stream, _) = listener.accept().await?;
                serve(stream, Reply::TextOnly(content)).await?;
            }
            Ok(())
        })
        .await?
    });
    Ok((format!("http://{address}"), server))
}

fn prompt_client(endpoint: &str) -> TestResult<ModelClient> {
    Ok(ModelClient::new(
        ModelOptions::local(endpoint, "same-model-name")?
            .with_response_format(ModelResponseFormat::PromptJson)
            .with_timeout(Duration::from_secs(3)),
    )?)
}

#[test]
fn text_only_backend_requires_explicit_prompt_json_before_consensus_acceptance() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let good = batch(&case, "value_proposed", "state", "WI");
            let (first, server_a) = text_backend(vec![good.clone(), good.clone()]).await?;
            let (second, server_b) = lane(vec![good])?;
            let constrained = [client(&first)?, client(&second)?];
            let failed = run_lanes(&fixture.store, &constrained, &options(), "schema").await?;
            check!(eq; failed.failed, 1);
            check!(eq; failed.accepted, 0);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            let configured = [prompt_client(&first)?, client(&second)?];
            let accepted =
                run_lanes(&fixture.store, &configured, &options(), "explicit-text").await?;
            server_a.await??;
            server_b.join().map_err(|_| "schema fixture panicked")??;
            check!(eq; accepted.failed, 0);
            check!(eq; accepted.accepted, 1);
            check!(eq; row(&fixture.store)?.value, "WI");
            check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            Ok(())
        })
}

#[test]
fn malformed_prompt_json_advice_remains_failed_without_extracting_embedded_json() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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
                let (fixture, case) = Fixture::school()?;
                let good = batch(&case, "value_proposed", "state", "WI");
                let malformed = malformed_advice(&good, scenario)?;
                let (first, server_a) = text_backend(vec![malformed]).await?;
                let (second, server_b) = lane(vec![good])?;
                let report = run_lanes(
                    &fixture.store,
                    &[prompt_client(&first)?, client(&second)?],
                    &options(),
                    "malformed-text",
                )
                .await?;
                server_a.await??;
                server_b.join().map_err(|_| "schema fixture panicked")??;
                check!(eq; report.failed, 1, "{scenario}");
                check!(eq; report.accepted, 0, "{scenario}");
                check!(eq; audit(&fixture.store)?["outcome"], "lane_failed", "{scenario}");
                check!(eq; audit(&fixture.store)?["lanes"][0]["status"], "failed", "{scenario}");
                check!(!row(&fixture.store)?.accepted, "{scenario}");
                check!(eq; state(&fixture.store)?, ReviewState::Retained, "{scenario}");
            }
            Ok(())
        })
}

#[test]
fn matching_prompt_json_proposals_cannot_introduce_an_invalid_jurisdiction() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let unsupported = batch(&case, "value_proposed", "state", "Atlantis");
            let (first, server_a) = text_backend(vec![unsupported.clone()]).await?;
            let (second, server_b) = text_backend(vec![unsupported]).await?;
            let report = run_lanes(
                &fixture.store,
                &[prompt_client(&first)?, prompt_client(&second)?],
                &options(),
                "unsupported-text",
            )
            .await?;
            server_a.await??;
            server_b.await??;
            check!(eq; report.failed, 0);
            check!(eq; report.accepted, 0);
            check!(eq; report.rejected, 1);
            check!(eq; audit(&fixture.store)?["outcome"], "refused");
            check!(!row(&fixture.store)?.accepted);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            Ok(())
        })
}

fn malformed_advice(good: &str, scenario: &str) -> TestResult<String> {
    match scenario {
        "prose" => return Ok(format!("The answer is {good}")),
        "trailing-prose" => return Ok(format!("{good}\nI inferred the missing facts.")),
        "balanced-fence" => return Ok(format!("```json\n{good}\n```")),
        "unclosed-fence" => return Ok(format!("```json\n{good}")),
        _ => {}
    }
    let mut value: serde_json::Value = serde_json::from_str(good)?;
    let (operation, field) = scenario.split_once(':').ok_or("malformation selector")?;
    let target = if matches!(field, "subject_id" | "verdicts") {
        &mut value
    } else {
        &mut value["verdicts"][0]
    };
    match operation {
        "missing" => {
            target.as_object_mut().ok_or("wire object")?.remove(field);
        }
        "null" => target[field] = serde_json::Value::Null,
        "unknown" => target["invented"] = serde_json::json!("hallucinated fact"),
        "kind" => target["kind"] = serde_json::json!("invented_verdict"),
        "confidence" => target["confidence"] = serde_json::from_str(field)?,
        other => return Err(format!("unknown malformation {other}").into()),
    }
    Ok(value.to_string())
}

#[test]
fn valid_prompt_json_insufficient_evidence_remains_answered_but_unaccepted() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (fixture, case) = Fixture::school()?;
            let insufficient = batch(&case, "insufficient_evidence", "", "");
            let (first, server_a) = text_backend(vec![insufficient.clone()]).await?;
            let (second, server_b) = text_backend(vec![insufficient]).await?;
            let report = run_lanes(
                &fixture.store,
                &[prompt_client(&first)?, prompt_client(&second)?],
                &options(),
                "insufficient-text",
            )
            .await?;
            server_a.await??;
            server_b.await??;
            check!(eq; report.failed, 0);
            check!(eq; report.accepted, 0);
            check!(eq; report.insufficient, 1);
            check!(eq; audit(&fixture.store)?["outcome"], "insufficient_evidence");
            check!(eq; audit(&fixture.store)?["lanes"][0]["status"], "answered");
            check!(eq; audit(&fixture.store)?["lanes"][1]["status"], "answered");
            check!(!row(&fixture.store)?.accepted);
            check!(eq; state(&fixture.store)?, ReviewState::Retained);
            Ok(())
        })
}
