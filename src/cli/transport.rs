use super::flow_control;
use anyhow::{bail, Context, Result};
use futures::TryStreamExt;
use restate_sdk::ingress::{ClientError, ReqwestClient};
use std::time::Duration;
use url::{Host, Url};

#[derive(serde::Deserialize)]
struct IngressFailure {
    message: String,
}

/// Restate reports terminal handler failures as a JSON body (`{"code":…,"message":…}`).
/// Surface that message instead of the bare status so operator commands explain why
/// they were rejected, for example a concurrency above the deployed worker capacity.
pub fn ingress_error(error: ClientError) -> anyhow::Error {
    let detail = error.response().and_then(|response| {
        serde_json::from_slice::<IngressFailure>(response.body())
            .ok()
            .map(|IngressFailure { message }| (response.status(), message))
    });
    match detail {
        Some((status, message)) => {
            anyhow::anyhow!("ingress returned HTTP status {status}: {message}")
        }
        None => error.into(),
    }
}

pub fn local_origin(value: &str) -> Result<Url> {
    let url = Url::parse(value)?;
    let local = match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        Some(Host::Domain(name)) => name == "localhost",
        None => false,
    };
    if !local
        || !matches!(url.scheme(), "http" | "https")
        || url.path() != "/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("control endpoints must be loopback HTTP origins without credentials or paths");
    }
    Ok(url)
}

pub fn client(origin: &str) -> Result<ReqwestClient> {
    let url = local_origin(origin)?;
    Ok(ReqwestClient::new(url.as_str().parse()?, http_client()?)?)
}

fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(300))
        .build()?)
}

pub async fn deploy(admin: &str, endpoint: &str) -> Result<serde_json::Value> {
    let admin = local_origin(admin)?;
    let endpoint = local_origin(endpoint)?;
    let client = http_client()?;
    flow_control::ensure_source_scope(&client, &admin).await?;
    let response = client
        .post(admin.join("deployments")?)
        .json(&serde_json::json!({"uri": endpoint.as_str()}))
        .send()
        .await?
        .error_for_status()?;
    let bytes = response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut bytes, chunk| async move {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > 65_536)
            {
                return Err(anyhow::anyhow!("deployment response exceeds 64 KiB"));
            }
            bytes.extend_from_slice(&chunk);
            Ok(bytes)
        })
        .await?;
    serde_json::from_slice(&bytes).context("decoding native deployment response")
}
