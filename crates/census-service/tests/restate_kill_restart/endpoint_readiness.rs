use crate::{Endpoint, Instant, POLL_INTERVAL};
use serde_json::Value;

const DISCOVERY_ACCEPT: &str = "application/vnd.restate.endpointmanifest.v4+json";
const MAX_READINESS_PROBES: usize = 240;

pub(super) fn failure(endpoint: &Endpoint, reason: &str) -> String {
    let captured = match std::fs::read_to_string(&endpoint.log_path) {
        Ok(log) => log,
        Err(error) => format!("captured stdout/stderr could not be read: {error}"),
    };
    format!(
        "endpoint pid={} address={} {reason}\n--- captured stdout/stderr ({}):\n{captured}",
        endpoint.guard.child.id(),
        endpoint.listen,
        endpoint.log_path.display(),
    )
}

pub(super) fn require_running(endpoint: &mut Endpoint) -> Result<(), String> {
    match endpoint.guard.child.try_wait() {
        Ok(None) => Ok(()),
        Ok(Some(status)) => Err(failure(
            endpoint,
            &format!("exited before readiness or registration: {status}"),
        )),
        Err(error) => Err(failure(
            endpoint,
            &format!("child exit status could not be read: {error}"),
        )),
    }
}

fn require_manifest(manifest: &Value) -> Result<(), String> {
    let services = manifest
        .get("services")
        .and_then(Value::as_array)
        .ok_or("discovery manifest carries no services array")?;
    let service = services
        .iter()
        .find(|service| service.get("name").and_then(Value::as_str) == Some("Consolidate"))
        .ok_or("discovery manifest does not advertise Consolidate")?;
    let handlers = service
        .get("handlers")
        .and_then(Value::as_array)
        .ok_or("Consolidate discovery manifest carries no handlers array")?;
    if !handlers
        .iter()
        .any(|handler| handler.get("name").and_then(Value::as_str) == Some("run"))
    {
        return Err("discovery manifest does not advertise Consolidate/run".into());
    }
    Ok(())
}

async fn discover(
    client: &reqwest::Client,
    endpoint: &Endpoint,
    deadline: Instant,
) -> Result<(), String> {
    let response = client
        .get(format!("http://{}/discover", endpoint.listen))
        .header("accept", DISCOVERY_ACCEPT)
        .timeout(deadline.saturating_duration_since(Instant::now()))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(format!("discovery answered {}", response.status()));
    }
    let manifest: Value = response
        .json()
        .await
        .map_err(|error| format!("discovery manifest was not JSON: {error}"))?;
    require_manifest(&manifest)
}

pub(super) async fn wait(endpoint: &mut Endpoint, deadline: Instant) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .build()
        .map_err(|error| failure(endpoint, &format!("discovery client failed: {error}")))?;
    let mut last = String::from("no discovery attempt made");
    for _ in 0..MAX_READINESS_PROBES {
        require_running(endpoint)?;
        if deadline.saturating_duration_since(Instant::now()).is_zero() {
            break;
        }
        let observed = discover(&client, endpoint, deadline).await;
        require_running(endpoint)?;
        match observed {
            Ok(()) if Instant::now() < deadline => return Ok(()),
            Ok(()) => last = String::from("discovery answered after the readiness deadline"),
            Err(reason) => last = reason,
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        tokio::time::sleep(POLL_INTERVAL.min(remaining)).await;
    }
    Err(failure(
        endpoint,
        &format!(
            "never answered live Consolidate/run discovery within the readiness budget: {last}"
        ),
    ))
}

#[path = "endpoint_readiness_tests.rs"]
mod tests;
