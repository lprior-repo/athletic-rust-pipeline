use super::super::{artifacts, GUEST};
use anyhow::{ensure, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::path::Path;

mod boundary;
mod captures;
mod effects;
mod injection;
mod input;
mod journal;
mod observe;
mod offline;
mod recovery;

pub(super) fn offline() -> Result<Value> {
    offline::read()
}

#[tracing::instrument(skip(client))]
pub(super) async fn quiescence(client: &Client) -> Result<Value> {
    let original = input::load()?;
    let finished = artifacts::json(&Path::new(GUEST).join("jurisdiction-recovery-finished.json"))?;
    recovery::verify_live(client, &original, &finished).await
}

#[tracing::instrument(skip(client))]
pub(super) async fn start(client: &Client) -> Result<Value> {
    let original = input::submit(client).await?;
    let boundary = boundary::wait(client, &original).await?;
    let evidence = json!({
        "original": original,
        "boundary": boundary,
        "reached_real_source_stage": true,
        "physical_request_in_flight_proven": false,
        "unproven_obligations": [
            "exact HTTP request/response/capture/parse fault subphase needs a production boundary signal"
        ]
    });
    artifacts::publish(
        &Path::new(GUEST).join("jurisdiction-recovery-boundary.json"),
        &evidence,
    )?;
    Ok(evidence)
}

#[tracing::instrument(skip(client))]
pub(super) async fn finish(client: &Client) -> Result<Value> {
    let original = input::load()?;
    let before = artifacts::json(&Path::new(GUEST).join("jurisdiction-recovery-boundary.json"))?;
    ensure!(
        before.get("original") == Some(&serde_json::to_value(&original)?),
        "original recovery request changed"
    );
    let clock = super::clock::clock()?;
    ensure!(
        original
            .clock
            .get("boot_id")
            .context("original boot absent")?
            != clock.get("boot_id").context("recovery boot absent")?,
        "guest boot did not change"
    );
    ensure!(
        original.clock.get("machine_id") == clock.get("machine_id"),
        "recovery machine changed"
    );
    let retained = injection::retained(&original, &before)?;
    let mut evidence = recovery::finish(client, &original, &before, clock).await?;
    evidence
        .as_object_mut()
        .context("recovery evidence malformed")?
        .insert("native_source_boundary_retained".to_owned(), retained);
    artifacts::publish(
        &Path::new(GUEST).join("jurisdiction-recovery-finished.json"),
        &evidence,
    )?;
    Ok(evidence)
}
