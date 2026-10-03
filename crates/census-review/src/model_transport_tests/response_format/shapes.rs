use super::*;
use crate::model::response::parse_batch;
use crate::ModelError;
use census_domain::model::{ReviewVerdict, ReviewVerdictKind, VerdictBatch};

#[derive(Debug, Clone, Copy)]
enum MalformedShape {
    PositionalEnvelope,
    PositionalVerdict,
    ObjectKind,
}

fn malformed_shape(good: &str, shape: MalformedShape) -> TestResult<String> {
    let mut value: serde_json::Value = serde_json::from_str(good)?;
    let verdict = &value["verdicts"][0];
    let positional = serde_json::json!([
        verdict["case_id"],
        verdict["kind"],
        verdict["field"],
        verdict["value"],
        verdict["confidence"],
        verdict["rationale"],
    ]);
    match shape {
        MalformedShape::PositionalEnvelope => {
            return Ok(serde_json::json!([value["subject_id"], [positional]]).to_string());
        }
        MalformedShape::PositionalVerdict => value["verdicts"][0] = positional,
        MalformedShape::ObjectKind => {
            value["verdicts"][0]["kind"] = serde_json::json!({"value_proposed": null})
        }
    }
    Ok(value.to_string())
}

#[test]
fn parser_rejects_positional_envelopes_verdicts_and_object_kinds() -> TestResult {
    let good = r#"{"subject_id":"s","verdicts":[{"case_id":"c","kind":"value_proposed","field":"state","value":"WI","confidence":80,"rationale":"retained evidence"}]}"#;
    for shape in [
        MalformedShape::PositionalEnvelope,
        MalformedShape::PositionalVerdict,
        MalformedShape::ObjectKind,
    ] {
        check!(
            matches!(
                parse_batch(&malformed_shape(good, shape)?),
                Err(ModelError::Content { .. })
            ),
            "{shape:?}"
        );
    }
    let object_verdict_envelope = format!(
        r#"["s",{}]"#,
        serde_json::from_str::<serde_json::Value>(good)?["verdicts"]
    );
    check!(matches!(
        parse_batch(&object_verdict_envelope),
        Err(ModelError::Content { .. })
    ));
    Ok(())
}

#[test]
fn parser_preserves_typed_object_replies_with_string_kinds() -> TestResult {
    for (kind, slug) in [
        (ReviewVerdictKind::ValueProposed, "value_proposed"),
        (
            ReviewVerdictKind::InsufficientEvidence,
            "insufficient_evidence",
        ),
    ] {
        let content = serde_json::json!({
            "subject_id": "s", "verdicts": [{
                "case_id": "c", "kind": slug, "field": "state", "value": "WI",
                "confidence": 100, "rationale": "retained evidence",
            }],
        })
        .to_string();
        check!(eq; parse_batch(&content)?,
        VerdictBatch {
            subject_id: "s".to_string(),
            verdicts: vec![ReviewVerdict {
                case_id: "c".to_string(),
                kind,
                field: Some("state".to_string()),
                value: Some("WI".to_string()),
                confidence: 100,
                rationale: "retained evidence".to_string(),
            }],
        });
    }
    Ok(())
}

#[test]
fn malformed_shapes_fail_before_acceptance_under_both_response_formats() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for format in [
                ModelResponseFormat::JsonSchema,
                ModelResponseFormat::PromptJson,
            ] {
                for shape in [
                    MalformedShape::PositionalEnvelope,
                    MalformedShape::PositionalVerdict,
                    MalformedShape::ObjectKind,
                ] {
                    let (fixture, case) = Fixture::school()?;
                    let good = batch(&case, "value_proposed", "state", "WI");
                    let (first, server_a) = lane(vec![malformed_shape(&good, shape)?])?;
                    let (second, server_b) = lane(vec![good])?;
                    let configured = ModelClient::new(
                        ModelOptions::local(&first, "same-model-name")?
                            .with_response_format(format)
                            .with_timeout(Duration::from_secs(3)),
                    )?;
                    let report = run_lanes(
                        &fixture.store,
                        &[configured, client(&second)?],
                        &options(),
                        "malformed-shape",
                    )
                    .await?;
                    server_a
                        .join()
                        .map_err(|_| "malformed fixture panicked")??;
                    server_b.join().map_err(|_| "valid fixture panicked")??;
                    check!(eq; report.failed, 1, "{format:?}: {shape:?}");
                    check!(eq; report.accepted, 0, "{format:?}: {shape:?}");
                    let retained = audit(&fixture.store)?;
                    check!(eq; retained["outcome"], "lane_failed", "{format:?}: {shape:?}");
                    check!(eq; retained["lanes"][0]["status"], "failed");
                    check!(eq; retained["lanes"][0]["batch"], serde_json::Value::Null);
                    check!(eq; retained["lanes"][1]["batch"]["verdicts"][0]["value"], "WI");
                    check!(!row(&fixture.store)?.accepted);
                    check!(eq; state(&fixture.store)?, ReviewState::Retained);
                }
            }
            Ok(())
        })
}

#[test]
fn typed_object_proposals_are_accepted_under_both_response_formats() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for format in [
                ModelResponseFormat::JsonSchema,
                ModelResponseFormat::PromptJson,
            ] {
                let (fixture, case) = Fixture::school()?;
                let good = batch(&case, "value_proposed", "state", "WI");
                let (first, server_a) = lane(vec![good.clone()])?;
                let (second, server_b) = lane(vec![good])?;
                let configured = ModelClient::new(
                    ModelOptions::local(&first, "same-model-name")?
                        .with_response_format(format)
                        .with_timeout(Duration::from_secs(3)),
                )?;
                let report = run_lanes(
                    &fixture.store,
                    &[configured, client(&second)?],
                    &options(),
                    "typed-object",
                )
                .await?;
                server_a.join().map_err(|_| "first fixture panicked")??;
                server_b.join().map_err(|_| "second fixture panicked")??;
                check!(eq; report.failed, 0, "{format:?}");
                check!(eq; report.accepted, 1, "{format:?}");
                check!(eq; audit(&fixture.store)?["outcome"], "agreement");
                check!(eq; row(&fixture.store)?.value, "WI");
                check!(row(&fixture.store)?.accepted);
                check!(eq; state(&fixture.store)?, ReviewState::Resolved);
            }
            Ok(())
        })
}
