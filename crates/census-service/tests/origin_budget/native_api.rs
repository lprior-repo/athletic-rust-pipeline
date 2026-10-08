use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use super::fixture::{Input, Reply};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Submission {
    #[serde(rename = "invocationId")]
    pub invocation_id: String,
    pub status: String,
}

pub(super) fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(60)).build()?)
}

pub(super) async fn request(request: reqwest::RequestBuilder, path: &Path) -> Result<Value> {
    let response = request.send().await?;
    let status = response.status();
    let mut response = response;
    let mut bytes = Vec::new();
    for _ in 0..4096 {
        let Some(chunk) = response.chunk().await? else {
            std::fs::write(path, &bytes)?;
            ensure!(status.is_success(), "native HTTP {status}; retained {}", path.display());
            return Ok(serde_json::from_slice(&bytes)?);
        };
        ensure!(bytes.len().saturating_add(chunk.len()) <= 1_048_576, "native reply exceeded 1MiB");
        bytes.extend_from_slice(&chunk);
    }
    Err(anyhow::anyhow!("native reply chunk limit exceeded"))
}

pub(super) async fn register(client: &reqwest::Client, admin: &str, endpoint: u16, root: &Path) -> Result<Value> {
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    let mut last = String::new();
    for _ in 0..240 {
        let probe = client.post(format!("{admin}/query")).header("accept", "application/json")
            .json(&json!({"query":"SELECT id FROM sys_invocation LIMIT 1"})).timeout(Duration::from_secs(2)).send().await;
        match probe {
            Ok(response) if response.status().is_success() => return registration(client, admin, endpoint, root).await,
            result => last = format!("{result:?}"),
        }
        tick.tick().await;
    }
    Err(anyhow::anyhow!("owned native node never ready: {last}"))
}

async fn registration(client: &reqwest::Client, admin: &str, endpoint: u16, root: &Path) -> Result<Value> {
    let deployment = request(client.post(format!("{admin}/deployments"))
        .json(&json!({"uri":format!("http://127.0.0.1:{endpoint}/")})), &root.with_extension("deployment.json")).await?;
    let inventory = request(client.get(format!("{admin}/deployments")), &root.with_extension("inventory.json")).await?;
    ensure!(inventory.to_string().contains("BudgetProbe"), "registered inventory lacks actual private handler: {inventory}");
    Ok(json!({"registration":deployment,"inventory":inventory}))
}

pub(super) async fn submit(client: &reqwest::Client, ingress: &str, key: &str, input: &Input, path: &Path) -> Result<Submission> {
    let value = request(client.post(format!("{ingress}/BudgetProbe/acquire/send"))
        .header("idempotency-key", key).json(input), path).await?;
    let submission: Submission = serde_json::from_value(value)?;
    ensure!(submission.invocation_id.starts_with("inv_") && submission.invocation_id.len() <= 128,
        "invalid original native invocation identity");
    ensure!(submission.status == "Accepted" || submission.status == "PreviouslyAccepted", "invalid submission disposition");
    Ok(submission)
}

pub(super) async fn attach(client: &reqwest::Client, ingress: &str, submitted: &Submission, path: &Path) -> Result<Reply> {
    let value = request(client.get(format!("{ingress}/restate/attach/{}", submitted.invocation_id)), path).await?;
    let reply: Reply = serde_json::from_value(value)?;
    let id = match &reply { Reply::Complete { invocation_id, .. } | Reply::Refused { invocation_id, .. } => invocation_id };
    ensure!(*id == submitted.invocation_id, "handler returned foreign invocation id");
    Ok(reply)
}

pub(super) async fn journal(client: &reqwest::Client, admin: &str, path: &Path, expected: &[&Submission]) -> Result<Value> {
    let query = "SELECT id, status, target_service_name, target_handler_name, completion_result FROM sys_invocation WHERE target_service_name = 'BudgetProbe' ORDER BY id";
    let value = request(client.post(format!("{admin}/query")).header("accept", "application/json").json(&json!({"query":query})), path).await?;
    let rows = value.get("rows").and_then(Value::as_array).ok_or_else(|| anyhow::anyhow!("native journal rows missing"))?;
    ensure!(rows.len() == expected.len(), "native original invocation census differs: {value}");
    for submission in expected {
        ensure!(rows.iter().filter(|row| row.get("id").and_then(Value::as_str) == Some(&submission.invocation_id)
            && row.get("status").and_then(Value::as_str) == Some("completed")).count() == 1,
            "original native invocation did not survive completed: {value}");
    }
    Ok(value)
}
