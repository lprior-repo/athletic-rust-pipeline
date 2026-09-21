//! Read-only observation commands: run status, rankings control state, and browser
//! session readiness, all read back through the Restate ingress.

use super::output::emit;
use super::transport;
use anyhow::Result;
use athletic_rust_pipeline::{
    domain::identity::EvidenceDigest,
    runtime::{
        browser_session::{BrowserSessionIngressClient, ReadinessRequest, BROWSER_SESSION_KEY},
        control::{PipelineControlIngressClient, RankingControlsInput},
        run::RunCoordinatorIngressClient,
    },
};
use restate_sdk::prelude::*;

#[tracing::instrument(skip_all, fields(command = "status"))]
pub(super) async fn run_status(ingress: &str, run: &str) -> Result<()> {
    let client = RunCoordinatorIngressClient::from_client(transport::client(ingress)?, "global");
    let response = client
        .status(Json(EvidenceDigest::parse(run)?))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&response.0)
}

fn rankings_input(run: &str) -> Result<RankingControlsInput> {
    Ok(RankingControlsInput {
        run: EvidenceDigest::parse(run)?,
    })
}

#[tracing::instrument(skip_all, fields(command = "rankings-status"))]
pub(super) async fn rankings_status(ingress: &str, run: &str) -> Result<()> {
    let input = rankings_input(run)?;
    let control = PipelineControlIngressClient::from_client(transport::client(ingress)?);
    let state = control
        .rankings_progress(Json(input))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&state.0)
}

#[tracing::instrument(skip_all, fields(command = "rankings-pause"))]
pub(super) async fn rankings_pause(ingress: &str, run: &str) -> Result<()> {
    let input = rankings_input(run)?;
    let control = PipelineControlIngressClient::from_client(transport::client(ingress)?);
    control
        .rankings_pause(Json(input))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({"status": "paused"}))
}

#[tracing::instrument(skip_all, fields(command = "rankings-resume"))]
pub(super) async fn rankings_resume(ingress: &str, run: &str) -> Result<()> {
    let input = rankings_input(run)?;
    let control = PipelineControlIngressClient::from_client(transport::client(ingress)?);
    control
        .rankings_resume(Json(input))
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({"status": "resumed"}))
}

#[tracing::instrument(skip_all, fields(command = "browser-start"))]
pub(super) async fn browser_start(ingress: &str) -> Result<()> {
    let client =
        BrowserSessionIngressClient::from_client(transport::client(ingress)?, BROWSER_SESSION_KEY);
    let submitted = client
        .await_ready(Json(ReadinessRequest { operator: true }))
        .send()
        .await
        .map_err(transport::ingress_error)?;
    emit(&serde_json::json!({"invocation_id": submitted.invocation_handle().invocation_id()}))
}

#[tracing::instrument(skip_all, fields(command = "browser-status"))]
pub(super) async fn browser_status(ingress: &str) -> Result<()> {
    let client =
        BrowserSessionIngressClient::from_client(transport::client(ingress)?, BROWSER_SESSION_KEY);
    let response = client
        .status()
        .call()
        .await
        .map_err(transport::ingress_error)?
        .into_body()
        .map_err(transport::ingress_error)?;
    emit(&response.0)
}
