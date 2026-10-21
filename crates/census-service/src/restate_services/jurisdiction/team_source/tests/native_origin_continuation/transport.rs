use super::{endpoint, NativeNode, TestResult};

pub(super) struct Reply {
    pub(super) invocation: String,
    pub(super) body: Vec<u8>,
}

pub(super) async fn assert_output(
    client: &reqwest::Client,
    node: &NativeNode,
    reply: &Reply,
) -> TestResult {
    let (status, output) = endpoint::read_output(client, &node.ingress, &reply.invocation).await?;
    check!(eq; status, reqwest::StatusCode::OK);
    check!(eq; output, reply.body);
    Ok(())
}

pub(super) async fn call<T: serde::Serialize>(
    client: &reqwest::Client,
    ingress: &str,
    service: &str,
    key: &str,
    handler: &str,
    idempotency: &str,
    request: &T,
) -> TestResult<Reply> {
    let response = client
        .post(target(ingress, service, key, handler)?)
        .header("idempotency-key", idempotency)
        .json(request)
        .send()
        .await?;
    let (status, reply) = read(response).await?;
    check!(eq; status, reqwest::StatusCode::OK, "native {service}/{handler} failed: {}", String::from_utf8_lossy(&reply.body));
    Ok(reply)
}

pub(super) async fn call_empty(
    client: &reqwest::Client,
    ingress: &str,
    service: &str,
    key: &str,
    handler: &str,
    idempotency: &str,
) -> TestResult<Reply> {
    let response = client
        .post(target(ingress, service, key, handler)?)
        .header("idempotency-key", idempotency)
        .send()
        .await?;
    let (status, reply) = read(response).await?;
    check!(eq; status, reqwest::StatusCode::OK, "native bodyless {service}/{handler} failed: {}", String::from_utf8_lossy(&reply.body));
    Ok(reply)
}

pub(super) async fn call_status<T: serde::Serialize>(
    client: &reqwest::Client,
    ingress: &str,
    service: &str,
    key: &str,
    handler: &str,
    idempotency: &str,
    request: &T,
) -> TestResult<(reqwest::StatusCode, Reply)> {
    let response = client
        .post(target(ingress, service, key, handler)?)
        .header("idempotency-key", idempotency)
        .json(request)
        .send()
        .await?;
    read(response).await
}

fn target(ingress: &str, service: &str, key: &str, handler: &str) -> TestResult<reqwest::Url> {
    let mut url = reqwest::Url::parse(ingress)?;
    url.path_segments_mut()
        .map_err(|_| "native ingress URL cannot contain path segments")?
        .extend(["restate", "call", service, key, handler]);
    Ok(url)
}

async fn read(response: reqwest::Response) -> TestResult<(reqwest::StatusCode, Reply)> {
    let status = response.status();
    let invocation = response
        .headers()
        .get("x-restate-id")
        .ok_or("native response omitted invocation ID")?
        .to_str()?
        .to_owned();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    Ok((
        status,
        Reply {
            invocation,
            body: body.to_vec(),
        },
    ))
}
