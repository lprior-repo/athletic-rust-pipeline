use super::input::{self, national::hold, Original};
use super::observe;
use anyhow::{ensure, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

mod apply;
mod cold;
pub(super) use cold::read as cold;
mod source;
mod witness;

pub(super) async fn before_reset(
    client: &Client,
    original: &Original,
    boundary: &Value,
) -> Result<Value> {
    let (call, previous) = witness::boundary(boundary)?;
    let control = hold::pause_jurisdiction(client, &original.id, &original.key).await?;
    let current = source::read_held(client, original).await?;
    witness::held(original, &call, &previous, &current)?;
    let national = national_held(client, original).await?;
    let evidence = json!({"original_invocation_id":original.id,"source_call":call,
        "parent_control":control,"observation":current,"national":national,
        "reason":"retain the original source fault boundary before admitting its later owned roster scope"});
    observe::publish("owned-parent-held", &evidence)?;
    Ok(evidence)
}

pub(super) async fn after_reset(
    client: &Client,
    original: &Original,
    before: &Value,
) -> Result<Value> {
    let (call, previous) =
        witness::boundary(before.get("boundary").context("original boundary absent")?)?;
    let held = before
        .get("owned_resource_parent_hold")
        .context("original parent hold absent")?;
    ensure!(
        held.get("original_invocation_id").and_then(Value::as_str) == Some(original.id.as_str()),
        "held parent identity differs"
    );
    let paused = serde_json::from_value(
        held.get("observation")
            .context("held observation absent")?
            .clone(),
    )?;
    witness::held(original, &call, &previous, &paused)?;
    national_held(client, original).await?;
    let settled = source::wait(client, original, &call, &paused).await?;
    let output = source::output(client, &call, &settled).await?;
    let application = apply::admit(client, original, &call, &settled).await?;
    let current = source::read_held(client, original).await?;
    witness::held(original, &call, &settled, &current)?;
    source::unchanged(&call, &settled, &current)?;
    ensure!(
        source::output(client, &call, &current).await? == output,
        "original source output changed during admission"
    );
    national_held(client, original).await?;
    let resume = hold::resume_jurisdiction(client, &original.id, &original.key).await?;
    let evidence = json!({"original_invocation_id":original.id,"original_source_call":call,
        "source_before_admission":settled,"source_after_admission":current,
        "original_source_output":output,"application":application,"resume":resume,
        "replacement_submitted":false,"original_source_restarted":false,
        "scope":"normal owned capture admission only; no source, school, roster or eligibility acceptance"});
    observe::publish("owned-admission-resumed", &evidence)?;
    Ok(evidence)
}

async fn national_held(client: &Client, original: &Original) -> Result<Value> {
    let id = input::text(&original.accepted, "national_invocation_id")?;
    let key = input::text(&original.accepted, "national_key")?;
    let current = hold::state(client, id).await?;
    ensure!(
        input::text(&current, "status")? == "paused"
            && input::text(&current, "target_service_name")? == "NationalCensus"
            && input::text(&current, "target_service_key")? == key
            && input::text(&current, "target_handler_name")? == "run"
            && current.get("restarted_from").is_none_or(Value::is_null),
        "original normal national orchestrator is not held"
    );
    for field in [
        "id",
        "pinned_deployment_id",
        "pinned_service_protocol_version",
        "retry_count",
    ] {
        ensure!(
            original
                .accepted
                .pointer(&format!("/national_pause/after/{field}"))
                == current.get(field),
            "original national hold changed {field}"
        );
    }
    Ok(current)
}
