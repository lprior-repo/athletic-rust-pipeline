use super::{admin, Evidence, NativeNode, TestResult, SERVICE};
use serde_json::{json, Value};
use std::net::SocketAddr;

pub(super) async fn manifest(client: &reqwest::Client, address: SocketAddr) -> TestResult<Value> {
    let response = client
        .get(format!("http://{address}/discover"))
        .header("accept", "application/vnd.restate.endpointmanifest.v4+json")
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    check!(eq; status, reqwest::StatusCode::OK, "native discovery failed: {}", String::from_utf8_lossy(&body));
    Ok(serde_json::from_slice(&body)?)
}

pub(super) async fn only_id(client: &reqwest::Client, node: &NativeNode) -> TestResult<String> {
    let response = client
        .get(format!("{}/deployments", node.admin))
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    check!(eq; status, reqwest::StatusCode::OK);
    let value: Value = serde_json::from_slice(&body)?;
    let deployments = value["deployments"]
        .as_array()
        .ok_or("native deployments absent")?;
    let [deployment] = deployments.as_slice() else {
        return Err(format!("expected one isolated deployment, got {value}").into());
    };
    Ok(deployment["id"]
        .as_str()
        .ok_or("native deployment ID absent")?
        .into())
}

pub(super) async fn read(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
) -> TestResult<Value> {
    let response = client
        .get(format!("{}/deployments/{id}", node.admin))
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    check!(eq; status, reqwest::StatusCode::OK, "native deployment read failed: {}", String::from_utf8_lossy(&body));
    Ok(serde_json::from_slice(&body)?)
}

pub(super) async fn update(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
    address: SocketAddr,
    dry_run: bool,
    evidence: &Evidence,
) -> TestResult<Value> {
    let response = client
        .patch(format!("{}/deployments/{id}", node.admin))
        .json(&json!({"uri": format!("http://{address}/"), "dry_run": dry_run, "overwrite": true}))
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    evidence.write(
        if dry_run {
            "rediscovery-dry-run.json"
        } else {
            "rediscovery-apply.json"
        },
        &json!({"status": status.as_u16(), "body": String::from_utf8_lossy(&body)}),
    )?;
    check!(eq; status, reqwest::StatusCode::OK, "native 1.7.10 deployment PATCH dry_run={dry_run} failed: {}", String::from_utf8_lossy(&body));
    let value: Value = serde_json::from_slice(&body)?;
    check!(eq; value["id"].as_str(), Some(id));
    check!(eq; value["uri"].as_str(), Some(format!("http://{address}/").as_str()));
    Ok(value)
}

pub(super) fn require_additive(old: &Value, new: &Value) -> TestResult {
    let old_services = old["services"]
        .as_array()
        .ok_or("old discovery services absent")?;
    let new_services = new["services"]
        .as_array()
        .ok_or("new discovery services absent")?;
    let ([old_service], [new_service]) = (old_services.as_slice(), new_services.as_slice()) else {
        return Err("controlled endpoint must expose exactly one service".into());
    };
    check!(eq; old_service["name"].as_str(), Some(SERVICE));
    let mut old_shape = old_service.clone();
    let mut new_shape = new_service.clone();
    old_shape
        .as_object_mut()
        .ok_or("old service shape")?
        .remove("handlers");
    new_shape
        .as_object_mut()
        .ok_or("new service shape")?
        .remove("handlers");
    check!(eq; old_shape, new_shape);
    let old_handlers = old_service["handlers"]
        .as_array()
        .ok_or("old handlers absent")?;
    let new_handlers = new_service["handlers"]
        .as_array()
        .ok_or("new handlers absent")?;
    check!(eq; old_handlers.len(), 1);
    check!(eq; new_handlers.len(), 2);
    let old_run = old_handlers.first().ok_or("old run absent")?;
    check!(eq; old_run["name"].as_str(), Some("run"));
    check!(
        new_handlers.contains(old_run),
        "existing handler discovery changed"
    );
    check!(new_handlers
        .iter()
        .any(|handler| handler["name"] == "added"));
    for field in ["minProtocolVersion", "maxProtocolVersion", "protocolMode"] {
        check!(eq; &old[field], &new[field]);
    }
    Ok(())
}

pub(super) fn require_pin(invocation: &Value, deployment: &str, key: &str) -> TestResult {
    check!(eq; invocation["pinned_deployment_id"].as_str(), Some(deployment));
    check!(eq; invocation["pinned_service_protocol_version"].as_u64(), Some(7));
    check!(eq; invocation["target"].as_str(), Some(format!("{SERVICE}/{key}/run").as_str()));
    Ok(())
}

pub(super) fn require_rediscovered(old: &Value, new: &Value) -> TestResult {
    for field in [
        "id",
        "uri",
        "created_at",
        "protocol_type",
        "http_version",
        "min_protocol_version",
        "max_protocol_version",
        "sdk_version",
        "additional_headers",
    ] {
        check!(eq; &old[field], &new[field]);
    }
    check!(eq; new["max_protocol_version"].as_u64(), Some(7));
    let old_services = old["services"]
        .as_array()
        .ok_or("old deployment services absent")?;
    let new_services = new["services"]
        .as_array()
        .ok_or("updated deployment services absent")?;
    let ([old_service], [new_service]) = (old_services.as_slice(), new_services.as_slice()) else {
        return Err("native deployment service inventory changed".into());
    };
    for field in ["name", "ty", "deployment_id"] {
        check!(eq; &old_service[field], &new_service[field]);
    }
    let old_handlers = old_service["handlers"]
        .as_array()
        .ok_or("old deployment handlers absent")?;
    let new_handlers = new_service["handlers"]
        .as_array()
        .ok_or("updated deployment handlers absent")?;
    check!(eq; old_handlers.len(), 1);
    check!(eq; new_handlers.len(), 2);
    check!(new_handlers.contains(old_handlers.first().ok_or("old deployment run absent")?));
    check!(new_handlers
        .iter()
        .any(|handler| handler["name"] == "added"));
    Ok(())
}

pub(super) async fn require_single_invocation(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
) -> TestResult {
    let rows = admin::query(client, node, "SELECT id FROM sys_invocation").await?;
    check!(eq; rows, vec![json!({"id": id})]);
    Ok(())
}
