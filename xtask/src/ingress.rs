use anyhow::{bail, Result};
use restate_sdk::ingress::{ClientError, RequestTarget, ReqwestClient};
use std::future::Future;
use std::time::Duration;
use url::{Host, Url};

pub const NODE_ORIGIN: &str = "http://127.0.0.1:18095/";

#[derive(serde::Deserialize)]
struct IngressFailure {
    message: String,
}

pub fn error(error: ClientError) -> anyhow::Error {
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

const READ_TIMEOUT: Duration = Duration::from_secs(120);

const JOB_TIMEOUT: Duration = Duration::from_secs(900);

pub fn client(origin: &str) -> Result<(String, ReqwestClient)> {
    build(origin, READ_TIMEOUT)
}

pub fn job_client(origin: &str) -> Result<(String, ReqwestClient)> {
    build(origin, JOB_TIMEOUT)
}

fn build(origin: &str, timeout: Duration) -> Result<(String, ReqwestClient)> {
    let url = Url::parse(origin)?;
    if !is_loopback_origin(&url) {
        bail!(
            "ingress origins must be loopback HTTP origins without credentials or paths: {origin}"
        );
    }
    let endpoint = url.as_str().to_string();
    Ok((
        endpoint.clone(),
        ReqwestClient::new(endpoint.parse()?, http_client(timeout)?)?,
    ))
}

pub fn announce(endpoint: &str, service: &str, handler: &str) {
    println!(
        "+ POST {endpoint}restate/call/{}",
        RequestTarget::service(service, handler)
    );
}

pub fn block_on<F: Future>(future: F) -> Result<F::Output> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(runtime.block_on(future))
}

fn is_loopback_origin(url: &Url) -> bool {
    let local = match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        Some(Host::Domain(name)) => name == "localhost",
        None => false,
    };
    local
        && matches!(url.scheme(), "http" | "https")
        && url.path() == "/"
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::is_loopback_origin;
    use url::Url;

    fn origin(value: &str) -> bool {
        Url::parse(value)
            .map(|url| is_loopback_origin(&url))
            .unwrap_or(false)
    }

    #[test]
    fn a_loopback_origin_is_accepted_and_anything_else_is_not() {
        assert!(origin("http://127.0.0.1:18095/"));
        assert!(origin("http://localhost:18095/"));
        assert!(origin("http://[::1]:18095/"));
        assert!(!origin("http://10.0.0.4:18095/"));
        assert!(!origin("https://restate.example.com/"));
        assert!(!origin("http://127.0.0.1:18095/admin"));
        assert!(!origin("http://user:pass@127.0.0.1:18095/"));
        assert!(!origin("http://127.0.0.1:18095/?x=1"));
    }
}
