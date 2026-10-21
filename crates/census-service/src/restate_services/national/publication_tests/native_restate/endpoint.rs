use super::super::TestResult;
use super::node::NativeNode;
use std::net::SocketAddr;
use std::time::Duration;

pub(super) async fn register_endpoint(
    client: &reqwest::Client,
    node: &NativeNode,
    endpoint: SocketAddr,
) -> TestResult {
    register_named_endpoint(client, node, endpoint, "Workbook", "run").await
}

pub(crate) async fn register_named_endpoint(
    client: &reqwest::Client,
    node: &NativeNode,
    endpoint: SocketAddr,
    service: &str,
    handler: &str,
) -> TestResult {
    let address = format!("http://{endpoint}/");
    let discovery = reqwest::Client::builder()
        .no_proxy()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let manifest = discovery
        .get(format!("{address}discover"))
        .header("accept", "application/vnd.restate.endpointmanifest.v4+json")
        .send()
        .await?;
    check!(eq; manifest.status(), reqwest::StatusCode::OK);
    let manifest: serde_json::Value = manifest.json().await?;
    require_manifest(&manifest, service, handler)?;
    let response = client
        .post(format!("{}/deployments", node.admin))
        .json(&serde_json::json!({ "uri": address }))
        .send()
        .await?;
    check!(
        response.status().is_success(),
        "Restate endpoint registration failed: {}",
        response.text().await?
    );
    Ok(())
}

fn require_manifest(manifest: &serde_json::Value, service: &str, handler: &str) -> TestResult {
    let services = manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .ok_or("endpoint discovery manifest has no services")?;
    let advertised = services
        .iter()
        .find(|entry| entry.get("name").and_then(serde_json::Value::as_str) == Some(service))
        .ok_or("endpoint discovery manifest does not advertise the required service")?;
    let handlers = advertised
        .get("handlers")
        .and_then(serde_json::Value::as_array)
        .ok_or("discovery manifest has no handlers")?;
    check!(
        handlers.iter().any(|entry| {
            entry.get("name").and_then(serde_json::Value::as_str) == Some(handler)
        }),
        "endpoint discovery manifest does not advertise the required handler"
    );
    Ok(())
}

pub(super) async fn call_workbook(
    client: &reqwest::Client,
    ingress: &str,
    workflow_key: &str,
    request: &crate::restate_services::WorkbookRequest,
) -> TestResult<(reqwest::StatusCode, String, Vec<u8>)> {
    let response = client
        .post(format!(
            "{ingress}/restate/call/Workbook/{workflow_key}/run"
        ))
        .json(request)
        .send()
        .await?;
    let status = response.status();
    let invocation_id = response
        .headers()
        .get("x-restate-id")
        .ok_or("Restate ingress response omitted x-restate-id")?
        .to_str()?
        .to_owned();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576, "Restate reply exceeds 1 MiB");
    Ok((status, invocation_id, body.to_vec()))
}

pub(crate) async fn read_output(
    client: &reqwest::Client,
    ingress: &str,
    invocation_id: &str,
) -> TestResult<(reqwest::StatusCode, Vec<u8>)> {
    let response = client
        .get(format!("{ingress}/restate/output/{invocation_id}"))
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576, "Restate output exceeds 1 MiB");
    Ok((status, body.to_vec()))
}
