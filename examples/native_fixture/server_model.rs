//! The model-origin endpoint: the OpenAI-shaped request the pipeline's model client posts, answered
//! with the deterministic verdict each scenario expects.

use super::server_request::{delayed, enabled, injected_failure, ActiveRequest};
use super::server_response::{failure, retryable_failure};
use super::FixtureState;
use crate::native_fixture::payloads;
use crate::native_fixture::scenarios::Scenario;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

pub(super) async fn model(State(state): State<FixtureState>, Json(body): Json<Value>) -> Response {
    let model_name = body
        .get("model")
        .and_then(Value::as_str)
        .map_or("unknown", |value| value)
        .to_owned();
    let input = body
        .get("messages")
        .and_then(Value::as_array)
        .and_then(|items| items.get(1))
        .and_then(|item| item.get("content"))
        .and_then(Value::as_str)
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok());
    let source = input.as_ref().and_then(|value| value.get("source"));
    let first = source
        .and_then(|value| value.get("fields"))
        .and_then(|value| value.get("Person First"))
        .and_then(Value::as_str)
        .map_or("", |value| value);
    let source_key = source
        .and_then(|value| value.get("source_key"))
        .and_then(Value::as_str)
        .map_or("unknown-source", |value| value);
    let case = payloads::scenario_for_text(first);
    if !enabled(&state, case) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let key = format!("model:{}:{model_name}:{source_key}", case.as_str());
    let Some(active) = ActiveRequest::begin(&state, "model", case, Some(model_name), key) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    delayed(&state).await;
    if injected_failure(&state, case) {
        return active.complete(retryable_failure("injected model failure"));
    }
    let output = match case {
        Scenario::AccessDenied => failure(StatusCode::FORBIDDEN, "fixture access denied"),
        Scenario::Malformed => {
            payloads::response(StatusCode::OK, "text/plain", b"not-json".to_vec())
        }
        Scenario::RetryExhaustion => retryable_failure("retry exhaustion"),
        Scenario::Ambiguous => payloads::model_response(
            json!({"decision":"unresolved","reason":"Two hard-eligible synthetic candidates remain indistinguishable."}),
        ),
        _ => payloads::model_response(
            json!({"decision":"unresolved","reason":"Fixture model is intentionally bypassed for deterministic cases."}),
        ),
    };
    active.complete(output)
}
