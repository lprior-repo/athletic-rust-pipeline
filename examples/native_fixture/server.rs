//! The fixture server: the state every route shares, the state's request descriptors, the routing
//! table that maps each path to its handler, and the listener that serves it.
//!
//! The handlers themselves live in the sibling modules declared here, one per origin: `server_source`
//! is the simulated Athletic.net source, `server_model` the model origin, `server_control` the
//! `/__fixture/*` control plane, `server_request` the per-request accounting entry and the gates a
//! handler passes through, and `server_response` the response writers those handlers share.

#[path = "server_control.rs"]
mod server_control;
#[path = "server_model.rs"]
mod server_model;
#[path = "server_request.rs"]
mod server_request;
#[path = "server_response.rs"]
mod server_response;
#[path = "server_source.rs"]
mod server_source;

use crate::native_fixture::scenarios::ScenarioSet;
use axum::{
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use server_control::{control_get, control_post, counters, reset};
use server_model::model;
use server_source::{bio, browser_home, fallback, search, team};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc, Mutex},
};
use tokio::{net::TcpListener, sync::Semaphore};

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
        .route("/", get(browser_home))
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
