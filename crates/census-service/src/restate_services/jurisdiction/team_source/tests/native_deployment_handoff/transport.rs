use super::{fixture::Retained, TestResult, SERVICE};

pub(super) const IDEMPOTENCY: &str = "native-deployment-handoff-original";

pub(super) async fn send(
    client: &reqwest::Client,
    ingress: &str,
    key: &str,
    input: &[u8],
) -> TestResult<String> {
    let response = client
        .post(target(ingress, "send", key)?)
        .header("idempotency-key", IDEMPOTENCY)
        .header("content-type", "application/json")
        .body(input.to_vec())
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    check!(eq; status, reqwest::StatusCode::ACCEPTED, "native send failed: {}", String::from_utf8_lossy(&body));
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    Ok(value["invocationId"]
        .as_str()
        .ok_or("native send invocation ID absent")?
        .into())
}

pub(super) async fn replay(
    client: &reqwest::Client,
    ingress: &str,
    key: &str,
    id: &str,
    input: &[u8],
    expected: &Retained,
) -> TestResult<Vec<u8>> {
    let response = client
        .post(target(ingress, "call", key)?)
        .header("idempotency-key", IDEMPOTENCY)
        .header("content-type", "application/json")
        .body(input.to_vec())
        .send()
        .await?;
    let status = response.status();
    let replay_id = response
        .headers()
        .get("x-restate-id")
        .ok_or("native replay invocation ID absent")?
        .to_str()?
        .to_owned();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    check!(eq; status, reqwest::StatusCode::OK, "native replay failed: {}", String::from_utf8_lossy(&body));
    check!(eq; replay_id, id);
    check!(eq; &serde_json::from_slice::<Retained>(&body)?, expected);
    Ok(body.to_vec())
}

fn target(ingress: &str, method: &str, key: &str) -> TestResult<reqwest::Url> {
    let mut url = reqwest::Url::parse(ingress)?;
    url.path_segments_mut()
        .map_err(|_| "native ingress cannot contain path segments")?
        .extend(["restate", method, SERVICE, key, "run"]);
    Ok(url)
}
