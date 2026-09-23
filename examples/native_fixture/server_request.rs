//! The per-request lifecycle: the accounting entry a handler holds for the duration of a request,
//! the drop-side bookkeeping that records its effect, and the gates — scenario enablement, the
//! injected-failure budget, the configured delay — it passes through on the way.

use super::{FixtureState, IdentityCounter};
use crate::native_fixture::scenarios::Scenario;
use axum::response::Response;
use std::sync::atomic::Ordering;
use tokio::time::{sleep, Duration};

pub(super) struct ActiveRequest {
    state: FixtureState,
    key: String,
    case: String,
    kind: String,
    completed: bool,
    failed: bool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}
impl ActiveRequest {
    pub(super) fn begin(
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
    pub(super) fn complete(mut self, response: Response) -> Response {
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

pub(super) async fn delayed(state: &FixtureState) {
    let delay = state.delay_ms.load(Ordering::Relaxed);
    if delay > 0 {
        sleep(Duration::from_millis(delay)).await;
    }
}

pub(super) fn injected_failure(state: &FixtureState, case: Scenario) -> bool {
    state
        .failures
        .lock()
        .ok()
        .and_then(|mut values| {
            values.get_mut(case.as_str()).map(|value| {
                if *value == 0 {
                    false
                } else {
                    *value = value.saturating_sub(1);
                    true
                }
            })
        })
        .is_some_and(|value| value)
}

pub(super) fn enabled(state: &FixtureState, case: Scenario) -> bool {
    state.scenarios.contains(case)
}

fn is_failure_case(case: &str) -> bool {
    matches!(
        case,
        "access-denied" | "malformed" | "retry-exhaustion" | "payload-limit"
    )
}
