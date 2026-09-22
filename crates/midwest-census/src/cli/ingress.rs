//! The Restate ingress transport the workflow-driving subcommands share.
//!
//! Same rules as the acquisition pipeline's own transport: the origin must be a loopback HTTP origin
//! without credentials or a path, redirects and proxies are off, and a terminal ingress failure is
//! reported with Restate's own message — that message is what tells an operator which flag to change
//! (`--revision` for a run that already exists with different parameters, for example).
//!
//! It is a second copy rather than a shared module because the two crates are separate compilation
//! units: the census crate is the one that owns the census services, and it may not depend on the
//! acquisition pipeline.

use anyhow::{bail, Result};
use restate_sdk::ingress::{ClientError, ReqwestClient};
use std::time::Duration;
use url::{Host, Url};

#[derive(serde::Deserialize)]
struct IngressFailure {
    message: String,
}

/// Restate reports terminal handler failures as a JSON body (`{"code":…,"message":…}`): surface that
/// message instead of the bare status, so a rejected submission explains itself.
pub(super) fn error(error: ClientError) -> anyhow::Error {
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

/// Build the ingress client for `origin`.
///
/// A non-loopback origin, a path, credentials or a query is refused before any work is asked for: the
/// census services run beside their store on this machine, and a mistyped origin must fail loudly
/// instead of driving some other deployment.
pub(super) fn client(origin: &str) -> Result<ReqwestClient> {
    let url = Url::parse(origin)?;
    if !is_loopback_origin(&url) {
        bail!(
            "ingress origins must be loopback HTTP origins without credentials or paths: {origin}"
        );
    }
    Ok(ReqwestClient::new(url.as_str().parse()?, http_client()?)?)
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

fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        // One buffered exchange, never a long poll: waiting for a run is the caller's loop, so a
        // wedged connection there is visible as a timeout on a poll instead of as silence.
        .timeout(Duration::from_secs(120))
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
        assert!(origin("http://127.0.0.1:18080/"));
        assert!(origin("http://localhost:18080/"));
        assert!(origin("http://[::1]:18080/"));
        assert!(!origin("http://10.0.0.4:18080/"));
        assert!(!origin("https://restate.example.com/"));
        assert!(!origin("http://127.0.0.1:18080/admin"));
        assert!(!origin("http://user:pass@127.0.0.1:18080/"));
        assert!(!origin("http://127.0.0.1:18080/?x=1"));
    }
}
