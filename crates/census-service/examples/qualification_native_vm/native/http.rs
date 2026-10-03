use super::super::{artifacts, GUEST};
use anyhow::{ensure, Context, Result};
use futures::TryStreamExt;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::path::Path;

const ADMIN: &str = super::ADMIN;
pub(super) const INGRESS: &str = "http://127.0.0.1:18095/";

#[tracing::instrument(skip(client, body))]
pub(super) async fn request(
    client: &Client,
    method: Method,
    url: &str,
    body: Option<&Value>,
) -> Result<Value> {
    let request = client
        .request(method, url)
        .header(reqwest::header::ACCEPT, "application/json");
    let request = match body {
        Some(body) => request.json(body),
        None => request,
    };
    let response = request.send().await?;
    let status = response.status();
    let invocation = response
        .headers()
        .get("x-restate-id")
        .map(|header| header.to_str().map(str::to_owned))
        .transpose()?;
    let bytes = response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut bytes, part| async move {
            ensure!(
                bytes.len().saturating_add(part.len()) <= 4 * 1024 * 1024,
                "native response exceeds four MiB"
            );
            bytes
                .try_reserve(part.len())
                .context("reserving bounded native response")?;
            bytes.extend_from_slice(&part);
            Ok(bytes)
        })
        .await?;
    let parsed = serde_json::from_slice::<Value>(&bytes);
    if parsed.is_err() {
        let path = Path::new(GUEST).join(format!("http-{}.body", artifacts::sha(&bytes)));
        artifacts::write(&path, &bytes)?;
        artifacts::append(
            &Path::new(GUEST).join("http.jsonl"),
            &json!({"url":url,"status":status.as_u16(),"invocation_id":invocation,"body_path":path,"at":artifacts::now()}),
        )?;
    }
    let value = parsed.with_context(|| {
        let prefix = bytes
            .get(..bytes.len().min(4096))
            .map_or(bytes.as_slice(), |value| value);
        format!(
            "native HTTP {status} at {url} is not JSON: {}",
            String::from_utf8_lossy(prefix)
        )
    })?;
    artifacts::append(
        &Path::new(GUEST).join("http.jsonl"),
        &json!({"url":url,"status":status.as_u16(),"invocation_id":invocation,"body":value,"at":artifacts::now()}),
    )?;
    ensure!(status.is_success(), "native HTTP failed: {status}: {value}");
    Ok(value)
}

#[tracing::instrument(skip(client))]
pub(super) async fn query(client: &Client, sql: &str) -> Result<Value> {
    request(
        client,
        Method::POST,
        &format!("{ADMIN}query"),
        Some(&json!({"query":sql})),
    )
    .await
}

pub(super) fn rows(value: &Value) -> Result<&[Value]> {
    if let Some(rows) = value.as_array() {
        return Ok(rows);
    }
    value
        .get("rows")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .context("query result rows absent")
}
