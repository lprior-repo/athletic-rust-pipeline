//! The simulated Athletic.net source origin: the browser landing document, the search endpoint, the
//! athlete-bio and team endpoints, and the athlete-profile fallback, each replaying one scenario's
//! byte-exact body.

use super::server_request::{delayed, enabled, injected_failure, ActiveRequest};
use super::server_response::{failure, retryable_failure, search_response};
use super::{FixtureState, SourceQuery};
use crate::native_fixture::payloads;
use crate::native_fixture::scenarios::Scenario;
use axum::{
    extract::{OriginalUri, Query, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::Value;

pub(super) async fn browser_home() -> axum::response::Html<&'static str> {
    axum::response::Html(
        "<!doctype html><html><head><title>Native browser fixture</title></head><body><script>document.cookie='native_fixture_js=enabled; Path=/; SameSite=Lax; Max-Age=86400';document.body.dataset.ready='true';</script><p>Native source fixture ready.</p></body></html>",
    )
}

pub(super) async fn search(State(state): State<FixtureState>, Json(body): Json<Value>) -> Response {
    let query = body
        .get("q")
        .and_then(Value::as_str)
        .map_or("", |value| value);
    let case = payloads::scenario_for_text(query);
    if !enabled(&state, case) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let start = body
        .get("start")
        .and_then(Value::as_u64)
        .map_or(0, |value| value);
    let key = format!("source:search:{}:{query}:{start}", case.as_str());
    let Some(active) = ActiveRequest::begin(&state, "source", case, None, key) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    delayed(&state).await;
    if injected_failure(&state, case) {
        return active.complete(retryable_failure("injected source failure"));
    }
    let output = match case {
        Scenario::AccessDenied => failure(StatusCode::FORBIDDEN, "fixture access denied"),
        Scenario::Malformed => {
            payloads::response(StatusCode::OK, "text/plain", b"not-json".to_vec())
        }
        Scenario::RetryExhaustion => retryable_failure("retry exhaustion"),
        Scenario::PayloadLimit => payloads::response(
            StatusCode::OK,
            "application/json",
            vec![b'x'; 33 * 1024 * 1024],
        ),
        Scenario::EmptySearch => search_response(Vec::new()),
        _ => {
            let sport = body
                .get("fq")
                .and_then(Value::as_str)
                .map_or("tf", |value| if value.contains("xc") { "xc" } else { "tf" });
            search_response(
                payloads::candidate_ids(case)
                    .into_iter()
                    .map(|id| payloads::search_row(id, case, sport))
                    .collect(),
            )
        }
    };
    active.complete(output)
}

pub(super) async fn bio(
    State(state): State<FixtureState>,
    Query(query): Query<SourceQuery>,
) -> Response {
    let id = query.athlete_id.map_or(0, |value| value);
    let case = payloads::scenario_for_id(id);
    if !enabled(&state, case) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let sport = query.sport.as_deref().map_or("tf", |value| value);
    let key = format!("source:bio:{id}:{sport}");
    let Some(active) = ActiveRequest::begin(&state, "source", case, None, key) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    delayed(&state).await;
    if injected_failure(&state, case) {
        return active.complete(retryable_failure("injected source failure"));
    }
    let output = match case {
        Scenario::AccessDenied => failure(StatusCode::FORBIDDEN, "fixture access denied"),
        Scenario::RetryExhaustion => retryable_failure("retry exhaustion"),
        Scenario::ProbeFailure if id == 1017 && sport == "tf" => {
            retryable_failure("identity probe source failure")
        }
        Scenario::PayloadLimit => payloads::response(
            StatusCode::OK,
            "application/json",
            vec![b'x'; 33 * 1024 * 1024],
        ),
        _ => payloads::response(
            StatusCode::OK,
            "application/json",
            payloads::bio_body(id, case, sport).into_bytes(),
        ),
    };
    active.complete(output)
}

pub(super) async fn team(
    State(state): State<FixtureState>,
    Query(query): Query<SourceQuery>,
) -> Response {
    let id = query.team.map_or(0, |value| value);
    let case = payloads::scenario_for_team(id);
    if !enabled(&state, case) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let sport = query.sport.as_deref().map_or("tf", |value| value);
    let season = query.season.map_or(0, |value| value);
    let key = format!("source:team:{id}:{sport}:{season}");
    let Some(active) = ActiveRequest::begin(&state, "source", case, None, key) else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    delayed(&state).await;
    if injected_failure(&state, case) {
        return active.complete(retryable_failure("injected source failure"));
    }
    let output = match case {
        Scenario::AccessDenied => failure(StatusCode::FORBIDDEN, "fixture access denied"),
        Scenario::RetryExhaustion => retryable_failure("retry exhaustion"),
        Scenario::PayloadLimit => payloads::response(
            StatusCode::OK,
            "application/json",
            vec![b'x'; 33 * 1024 * 1024],
        ),
        _ => payloads::response(
            StatusCode::OK,
            "application/json",
            payloads::team_body(id, case).into_bytes(),
        ),
    };
    active.complete(output)
}

pub(super) async fn fallback(
    method: Method,
    OriginalUri(uri): OriginalUri,
    State(state): State<FixtureState>,
) -> Response {
    if method == Method::GET && uri.path().starts_with("/athlete/") {
        let parts = uri.path().split('/').collect::<Vec<_>>();
        let id = parts
            .get(2)
            .and_then(|value| value.parse::<u64>().ok())
            .map_or(0, |value| value);
        let case = payloads::scenario_for_id(id);
        if !enabled(&state, case) {
            return StatusCode::NOT_FOUND.into_response();
        }
        let key = format!("source:profile:{id}:{}", uri.path());
        let Some(active) = ActiveRequest::begin(&state, "source", case, None, key) else {
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        };
        delayed(&state).await;
        if injected_failure(&state, case) {
            return active.complete(retryable_failure("injected profile failure"));
        }
        return active.complete(payloads::response(
            StatusCode::OK,
            "text/html; charset=utf-8",
            payloads::profile_html(id, case).into_bytes(),
        ));
    }
    StatusCode::NOT_FOUND.into_response()
}
