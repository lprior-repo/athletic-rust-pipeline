use super::super::super::super::{artifacts, GUEST};
use super::super::super::http::request;
use super::super::journal::SourceCall;
use super::{input, Original};
use anyhow::{ensure, Context, Result};
use census_store::Application;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::path::Path;

pub(super) async fn admit(
    client: &Client,
    original: &Original,
    call: &SourceCall,
    settled: &super::super::observe::Observation,
) -> Result<Value> {
    let root = Path::new(GUEST);
    let intent_path = root.join("jurisdiction-owned-admission-intent.json");
    ensure!(!intent_path.exists(), "owned admission was already attempted; uncertain original acknowledgement requires inspection");
    let target = input::target(
        "call",
        "TeamsSource",
        &call.child_key,
        "admit_public_resources",
    )?;
    let body = serde_json::to_value(&call.request)?;
    let admission_request_bytes = serde_json::to_vec(&body)?;
    let original_source_request_bytes = super::source::input_bytes(call, settled)?;
    ensure!(
        admission_request_bytes.len() <= 64 * 1024,
        "admission request exceeds 64 KiB"
    );
    let intent = json!({"parent_invocation_id":original.id,"original_source_call":call,
        "url":target,"request":body,"admission_request_bytes":admission_request_bytes,
        "original_source_request_bytes":original_source_request_bytes,"new_original_source_submission":false});
    artifacts::write(&intent_path, &serde_json::to_vec_pretty(&intent)?)?;
    let value = request(client, Method::POST, &target, Some(&body)).await?;
    let application: Application = serde_json::from_value(value.clone())
        .context("normal owned admission acknowledgement malformed")?;
    validate(&application)?;
    let evidence = json!({"intent":intent,"application":value,
        "interpretation":"server acknowledgement shape only; exact protected owner/receipt/root binding requires the existing drained cold oracle",
        "protected_binding_qualified":false});
    artifacts::publish(
        &root.join("jurisdiction-owned-admission-acknowledged.json"),
        &evidence,
    )?;
    Ok(evidence)
}

fn validate(application: &Application) -> Result<()> {
    let receipt = application.receipt();
    ensure!(
        receipt
            .operation
            .starts_with("public_resource_capture_v1:declare:")
            && receipt.appended == 0
            && !receipt.credited,
        "owned admission acknowledgement differs from zero-effect declaration"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
