use anyhow::{ensure, Context, Result};
use futures::TryStreamExt;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

use super::artifacts::{append, now};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub method: String,
    pub url: String,
    pub sent_at: String,
    pub received_at: String,
    pub status: u16,
    pub error_source: Option<String>,
    pub invocation_id: Option<String>,
    pub body: Value,
}

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()?)
}

#[tracing::instrument(skip(client, root, body))]
pub async fn request(
    client: &Client,
    root: &Path,
    method: Method,
    url: &str,
    body: Option<&Value>,
) -> Result<Observation> {
    let parsed = url::Url::parse(url)?;
    ensure!(
        parsed.host_str() == Some("127.0.0.1") && parsed.scheme() == "http",
        "qualification HTTP inspection requires literal-loopback HTTP"
    );
    let sent_at = now()?;
    append(
        &root.join("http-observations.jsonl"),
        &json!({"event":"request_start", "method":method.as_str(), "url":url, "sent_at":sent_at}),
    )?;
    let request = client
        .request(method.clone(), url)
        .header("accept", "application/json");
    let request = match body {
        Some(value) => request.json(value),
        None => request,
    };
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            append(
                &root.join("http-observations.jsonl"),
                &json!({"event":"transport_error", "method":method.as_str(), "url":url, "sent_at":sent_at, "failed_at":now()?, "error":error.to_string()}),
            )?;
            return Err(error.into());
        }
    };
    let status = response.status().as_u16();
    let error_source = header(&response, "x-restate-error-source")?;
    let invocation_id = header(&response, "x-restate-id")?;
    let body = read_body(response).await?;
    let observation = Observation {
        method: method.to_string(),
        url: url.to_string(),
        sent_at,
        received_at: now()?,
        status,
        error_source,
        invocation_id,
        body,
    };
    append(
        &root.join("http-observations.jsonl"),
        &serde_json::to_value(&observation)?,
    )?;
    Ok(observation)
}

#[tracing::instrument(skip(response))]
async fn read_body(response: reqwest::Response) -> Result<Value> {
    let bytes = response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut bytes, part| async move {
            if bytes.len().saturating_add(part.len()) > 4 * 1024 * 1024 {
                return Err(anyhow::anyhow!("native response exceeds four MiB"));
            }
            bytes.extend_from_slice(&part);
            Ok(bytes)
        })
        .await;
    let bytes = bytes.context("reading bounded native response")?;
    Ok(match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(_) => json!({"non_json_body":String::from_utf8_lossy(&bytes)}),
    })
}

fn header(response: &reqwest::Response, name: &str) -> Result<Option<String>> {
    response
        .headers()
        .get(name)
        .map(|value| value.to_str().map(str::to_string).map_err(Into::into))
        .transpose()
}

#[tracing::instrument(skip(client, root))]
pub async fn query(client: &Client, root: &Path, admin: &str, sql: &str) -> Result<Value> {
    let observation = request(
        client,
        root,
        Method::POST,
        &format!("{admin}query"),
        Some(&json!({"query":sql})),
    )
    .await?;
    ensure!(
        (200..300).contains(&observation.status),
        "native query failed: {} {}",
        observation.status,
        observation.body
    );
    Ok(observation.body)
}
