use super::super::TestResult;
use super::node::NativeNode;
use std::net::SocketAddr;
use std::time::Duration;

pub(super) async fn register_endpoint(
    client: &reqwest::Client,
    node: &NativeNode,
    endpoint: SocketAddr,
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
    require_workbook_manifest(&manifest)?;
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

fn require_workbook_manifest(manifest: &serde_json::Value) -> TestResult {
    let services = manifest
        .get("services")
        .and_then(serde_json::Value::as_array)
        .ok_or("endpoint discovery manifest has no services")?;
    let workbook = services
        .iter()
        .find(|service| service.get("name").and_then(serde_json::Value::as_str) == Some("Workbook"))
        .ok_or("endpoint discovery manifest does not advertise Workbook")?;
    let handlers = workbook
        .get("handlers")
        .and_then(serde_json::Value::as_array)
        .ok_or("Workbook discovery manifest has no handlers")?;
    check!(
        handlers.iter().any(|handler| {
            handler.get("name").and_then(serde_json::Value::as_str) == Some("run")
        }),
        "endpoint discovery manifest does not advertise Workbook/run"
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

pub(super) async fn read_output(
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
