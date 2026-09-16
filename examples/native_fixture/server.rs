use super::{
    payloads,
    scenarios::{Scenario, ScenarioSet},
};
use axum::{
    extract::{OriginalUri, Query, State},
    http::{header, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tokio::{
    net::TcpListener,
    sync::Semaphore,
    time::{sleep, Duration},
};

#[derive(Clone)]
pub struct FixtureState {
    pub scenarios: ScenarioSet,
    pub delay_ms: Arc<AtomicU64>,
    pub permits: Arc<Semaphore>,
    pub max_concurrency: usize,
    pub counters: Arc<Mutex<Counters>>,
    pub failures: Arc<Mutex<BTreeMap<String, u32>>>,
}
#[derive(Debug, Default, Clone, Serialize)]
pub struct Counters {
    pub source_requests: u64,
    pub model_requests: u64,
    pub source_completed_effects: u64,
    pub model_completed_effects: u64,
    pub active_requests: u64,
    pub max_active_requests: u64,
    pub per_case: BTreeMap<String, CaseCounter>,
    pub identities: BTreeMap<String, IdentityCounter>,
}
#[derive(Debug, Default, Clone, Serialize)]
pub struct CaseCounter {
    pub source_calls: u64,
    pub model_calls: u64,
    pub completed_effects: u64,
    pub failures: u64,
    pub in_flight: u64,
    pub max_in_flight: u64,
}
#[derive(Debug, Default, Clone, Serialize)]
pub struct IdentityCounter {
    pub kind: String,
    pub case: String,
    pub model: Option<String>,
    pub calls: u64,
    pub completed_effects: u64,
    pub in_flight: u64,
    pub max_in_flight: u64,
}
#[derive(Debug, Deserialize, Default)]
pub struct ControlPatch {
    pub delay_ms: Option<u64>,
    pub failure_budget: Option<BTreeMap<String, u32>>,
}
#[derive(Debug, Deserialize)]
pub struct SourceQuery {
    #[serde(rename = "athleteId")]
    pub athlete_id: Option<u64>,
    pub sport: Option<String>,
    pub team: Option<u64>,
    pub season: Option<u16>,
}

struct ActiveRequest {
    state: FixtureState,
    key: String,
    case: String,
    kind: String,
    completed: bool,
    failed: bool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}
impl ActiveRequest {
    fn begin(
        state: &FixtureState,
        kind: &str,
        case: Scenario,
        model: Option<String>,
        key: String,
    ) -> Option<Self> {
        let permit = state.permits.clone().try_acquire_owned().ok()?;
        let case_name = case.as_str().to_owned();
        let mut counters = state.counters.lock().ok()?;
        counters.active_requests = counters.active_requests.saturating_add(1);
        counters.max_active_requests = counters.max_active_requests.max(counters.active_requests);
        if kind == "source" {
            counters.source_requests = counters.source_requests.saturating_add(1);
        } else {
            counters.model_requests = counters.model_requests.saturating_add(1);
        }
        let entry = counters.per_case.entry(case_name.clone()).or_default();
        entry.in_flight = entry.in_flight.saturating_add(1);
        entry.max_in_flight = entry.max_in_flight.max(entry.in_flight);
        if kind == "source" {
            entry.source_calls = entry.source_calls.saturating_add(1);
        } else {
            entry.model_calls = entry.model_calls.saturating_add(1);
        }
        let identity = counters
            .identities
            .entry(key.clone())
            .or_insert_with(|| IdentityCounter {
                kind: kind.to_owned(),
                case: case_name.clone(),
                model,
                ..IdentityCounter::default()
            });
        identity.calls = identity.calls.saturating_add(1);
        identity.in_flight = identity.in_flight.saturating_add(1);
        identity.max_in_flight = identity.max_in_flight.max(identity.in_flight);
        drop(counters);
        Some(Self {
            state: state.clone(),
            key,
            case: case_name,
            kind: kind.to_owned(),
            completed: false,
            failed: false,
            _permit: permit,
        })
    }
    fn complete(mut self, response: Response) -> Response {
        self.failed = !response.status().is_success() || is_failure_case(&self.case);
        self.completed = true;
        response
    }
}
impl Drop for ActiveRequest {
    fn drop(&mut self) {
        if let Ok(mut counters) = self.state.counters.lock() {
            counters.active_requests = counters.active_requests.saturating_sub(1);
            if let Some(entry) = counters.per_case.get_mut(&self.case) {
                entry.in_flight = entry.in_flight.saturating_sub(1);
                if self.completed {
                    entry.completed_effects = entry.completed_effects.saturating_add(1);
                    if self.failed {
                        entry.failures = entry.failures.saturating_add(1);
                    }
                }
            }
            if self.completed {
                if let Some(identity) = counters.identities.get_mut(&self.key) {
                    identity.in_flight = identity.in_flight.saturating_sub(1);
                    identity.completed_effects = identity.completed_effects.saturating_add(1);
                }
            } else if let Some(identity) = counters.identities.get_mut(&self.key) {
                identity.in_flight = identity.in_flight.saturating_sub(1);
            }
            if self.completed {
                if self.kind == "source" {
                    counters.source_completed_effects =
                        counters.source_completed_effects.saturating_add(1);
                } else {
                    counters.model_completed_effects =
                        counters.model_completed_effects.saturating_add(1);
                }
            }
        }
    }
}
pub fn state(scenarios: ScenarioSet, delay_ms: u64, max_concurrency: usize) -> FixtureState {
    let max_concurrency = max_concurrency.clamp(1, 128);
    FixtureState {
        scenarios,
        delay_ms: Arc::new(AtomicU64::new(delay_ms.min(60_000))),
        permits: Arc::new(Semaphore::new(max_concurrency)),
        max_concurrency,
        counters: Arc::new(Mutex::new(Counters::default())),
        failures: Arc::new(Mutex::new(BTreeMap::new())),
    }
}

pub async fn serve(state: FixtureState, bind: SocketAddr) -> anyhow::Result<()> {
    let router = Router::new()
        .route("/Search.aspx/runSearch", post(search))
        .route("/api/v1/AthleteBio/GetAthleteBioData", get(bio))
        .route("/api/v1/TeamNav/Team", get(team))
        .route("/v1/chat/completions", post(model))
        .route("/__fixture/counters", get(counters))
        .route("/__fixture/control", get(control_get).post(control_post))
        .route("/__fixture/reset", post(reset))
        .fallback(fallback)
        .with_state(state);
    let listener = TcpListener::bind(bind).await?;
    tracing::info!(%bind, "native fixture listening");
    axum::serve(listener, router).await?;
    Ok(())
}
async fn search(State(state): State<FixtureState>, Json(body): Json<Value>) -> Response {
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
async fn bio(State(state): State<FixtureState>, Query(query): Query<SourceQuery>) -> Response {
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
async fn team(State(state): State<FixtureState>, Query(query): Query<SourceQuery>) -> Response {
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
async fn model(State(state): State<FixtureState>, Json(body): Json<Value>) -> Response {
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
async fn fallback(
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
async fn counters(State(state): State<FixtureState>) -> Json<Counters> {
    Json(
        state
            .counters
            .lock()
            .map_or_else(|_| Counters::default(), |value| value.clone()),
    )
}
async fn control_get(State(state): State<FixtureState>) -> Json<Value> {
    let failures = state
        .failures
        .lock()
        .map_or_else(|_| BTreeMap::new(), |value| value.clone());
    Json(
        json!({"scenarios": state.scenarios.scenarios.iter().map(|value| value.as_str()).collect::<Vec<_>>(), "delay_ms": state.delay_ms.load(Ordering::Relaxed), "max_concurrency": state.max_concurrency, "failure_budget": failures}),
    )
}
async fn control_post(
    State(state): State<FixtureState>,
    Json(patch): Json<ControlPatch>,
) -> Json<Value> {
    if let Some(delay) = patch.delay_ms {
        state.delay_ms.store(delay.min(60_000), Ordering::Relaxed);
    }
    if let Some(failures) = patch.failure_budget {
        if let Ok(mut target) = state.failures.lock() {
            *target = failures;
        }
    }
    control_get(State(state)).await
}
async fn reset(State(state): State<FixtureState>) -> StatusCode {
    if let Ok(mut counters) = state.counters.lock() {
        *counters = Counters::default();
    }
    if let Ok(mut failures) = state.failures.lock() {
        failures.clear();
    }
    StatusCode::NO_CONTENT
}
async fn delayed(state: &FixtureState) {
    let delay = state.delay_ms.load(Ordering::Relaxed);
    if delay > 0 {
        sleep(Duration::from_millis(delay)).await;
    }
}
fn injected_failure(state: &FixtureState, case: Scenario) -> bool {
    state
        .failures
        .lock()
        .ok()
        .and_then(|mut values| {
            values.get_mut(case.as_str()).map(|value| {
                if *value == 0 {
                    false
                } else {
                    *value -= 1;
                    true
                }
            })
        })
        .is_some_and(|value| value)
}
fn enabled(state: &FixtureState, case: Scenario) -> bool {
    state.scenarios.contains(case)
}
fn failure(status: StatusCode, message: &str) -> Response {
    payloads::response(
        status,
        "application/json",
        json!({"error":message}).to_string().into_bytes(),
    )
}
fn retryable_failure(message: &str) -> Response {
    let mut response = failure(StatusCode::SERVICE_UNAVAILABLE, message);
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("0"));
    response
}
fn search_response(rows: Vec<String>) -> Response {
    let html = format!("<table><tbody>{}</tbody></table>", rows.join(""));
    let body = json!({"d":{"results":html,"count":rows.len(),"pager":"","runTime":1}})
        .to_string()
        .into_bytes();
    payloads::response(StatusCode::OK, "application/json", body)
}
fn is_failure_case(case: &str) -> bool {
    matches!(
        case,
        "access-denied" | "malformed" | "retry-exhaustion" | "payload-limit"
    )
}
