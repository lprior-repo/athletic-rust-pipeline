//! The fixture's control plane: the counters snapshot, the scenario/delay/failure-budget view and
//! its patch endpoint, and the reset that clears both.

use super::{ControlPatch, Counters, FixtureState};
use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::{collections::BTreeMap, sync::atomic::Ordering};

pub(super) async fn counters(State(state): State<FixtureState>) -> Json<Counters> {
    Json(
        state
            .counters
            .lock()
            .map_or_else(|_| Counters::default(), |value| value.clone()),
    )
}

pub(super) async fn control_get(State(state): State<FixtureState>) -> Json<Value> {
    let failures = state
        .failures
        .lock()
        .map_or_else(|_| BTreeMap::new(), |value| value.clone());
    Json(
        json!({"scenarios": state.scenarios.scenarios.iter().map(|value| value.as_str()).collect::<Vec<_>>(), "delay_ms": state.delay_ms.load(Ordering::Relaxed), "max_concurrency": state.max_concurrency, "failure_budget": failures}),
    )
}

pub(super) async fn control_post(
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

pub(super) async fn reset(State(state): State<FixtureState>) -> StatusCode {
    if let Ok(mut counters) = state.counters.lock() {
        *counters = Counters::default();
    }
    if let Ok(mut failures) = state.failures.lock() {
        failures.clear();
    }
    StatusCode::NO_CONTENT
}
