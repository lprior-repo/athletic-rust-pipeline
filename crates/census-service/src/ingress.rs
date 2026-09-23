//! The Restate ingress transport this process talks to its own node through.
//!
//! Two callers share it: the workflow-driving subcommands (submit a run, watch it, read a durable
//! state) and the service shell, whose census fetcher reaches the `BrowserSession` object over the
//! same ingress. Both need one origin rule — loopback, no credentials, no path — and two copies of
//! it is how the two drift apart.
//!
//! Same rules as the acquisition pipeline's own transport: the origin must be a loopback HTTP origin
//! without credentials or a path, redirects and proxies are off, and a terminal ingress failure is
//! reported with Restate's own message — that message is what tells an operator which flag to change
//! (`--revision` for a run that already exists with different parameters, for example).

use anyhow::{bail, Result};
use restate_sdk::ingress::{ClientError, ReqwestClient};
use std::time::Duration;
use url::{Host, Url};

/// Ingress origin of the local census deployment: the Restate node that runs beside `census-serve`
/// on this machine, which is what `deploy/systemd/restate-server.service` starts. Every command
/// submits here unless the operator names another origin; the switch to the offline path is `--store`
/// and never a different default.
pub const DEFAULT_ORIGIN: &str = "http://127.0.0.1:18095/";

#[derive(serde::Deserialize)]
struct IngressFailure {
    message: String,
}

/// Restate reports terminal handler failures as a JSON body (`{"code":…,"message":…}`): surface that
/// message instead of the bare status, so a rejected submission explains itself.
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

/// The budget for a call that only reads: a status, a durable state, the last report, or one poll of a
/// run that is still going.
///
/// Waiting for a run is the caller's own loop, so a wedged connection is visible as a timeout on one
/// poll instead of as silence — and a read that has not answered in this long is wedged.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// The budget for a call that *runs* a job in the service: the append-log consolidation, the census
/// report, the best-mark reduction and the workbook each hold the store for minutes at full size. The
/// client must not abandon a call the deployment is still executing — the invocation keeps running
/// either way, and a client that gave up cannot report what it produced.
const JOB_TIMEOUT: Duration = Duration::from_secs(900);

/// Build the ingress client for `origin` with the read budget ([`READ_TIMEOUT`]).
pub fn client(origin: &str) -> Result<ReqwestClient> {
    build(origin, READ_TIMEOUT)
}

/// Build the ingress client for `origin` with the job budget ([`JOB_TIMEOUT`]).
///
/// Only the calls that own the store's writer for minutes use this: a submit's acceptance and every
/// read are answered in milliseconds, and a long budget there would hide a wedged connection.
pub fn job_client(origin: &str) -> Result<ReqwestClient> {
    build(origin, JOB_TIMEOUT)
}

/// Validate `origin` and build its client.
///
/// An empty origin is refused by name: a command that drives the service has no store to fall back to,
/// and `Url::parse`'s "relative URL without a base" would not tell an operator which flag to fix. A
/// non-loopback origin, a path, credentials or a query is refused for the same reason the census
/// services refuse them: they run beside their store on this machine, and a mistyped origin must fail
/// loudly instead of driving some other deployment.
fn build(origin: &str, timeout: Duration) -> Result<ReqwestClient> {
    if origin.trim().is_empty() {
        bail!(
            "--ingress must name the Restate ingress origin (the local census node is {DEFAULT_ORIGIN}): \
             this command drives the running census service, it does not open the store itself"
        );
    }
    let url = Url::parse(origin)?;
    if !is_loopback_origin(&url) {
        bail!(
            "ingress origins must be loopback HTTP origins without credentials or paths: {origin}"
        );
    }
    Ok(ReqwestClient::new(
        url.as_str().parse()?,
        http_client(timeout)?,
    )?)
}

/// The origin a command addresses: the one it was handed, or the local deployment's
/// [`DEFAULT_ORIGIN`] when the flag was left out.
pub fn origin(given: Option<&str>) -> &str {
    given.unwrap_or(DEFAULT_ORIGIN)
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

/// The HTTP transport under the ingress client: no proxy, no redirects, and the caller's own budget.
fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::{client, is_loopback_origin, origin, DEFAULT_ORIGIN};
    use url::Url;

    fn loopback(value: &str) -> bool {
        Url::parse(value)
            .map(|url| is_loopback_origin(&url))
            .unwrap_or(false)
    }

    #[test]
    fn a_loopback_origin_is_accepted_and_anything_else_is_not() {
        assert!(loopback("http://127.0.0.1:18095/"));
        assert!(loopback("http://localhost:18095/"));
        assert!(loopback("http://[::1]:18095/"));
        assert!(!loopback("http://10.0.0.4:18095/"));
        assert!(!loopback("https://restate.example.com/"));
        assert!(!loopback("http://127.0.0.1:18095/admin"));
        assert!(!loopback("http://user:pass@127.0.0.1:18095/"));
        assert!(!loopback("http://127.0.0.1:18095/?x=1"));
    }

    /// The deployment's own origin is the default; an origin the operator names wins, and a blank one
    /// fails naming `--ingress` rather than falling back to anything.
    #[test]
    fn an_absent_origin_is_the_local_deployment_and_a_blank_one_is_refused() {
        assert_eq!(origin(None), DEFAULT_ORIGIN);
        assert_eq!(
            origin(Some("http://127.0.0.1:19095/")),
            "http://127.0.0.1:19095/"
        );
        let Err(error) = client("  ") else {
            panic!("a blank origin must be refused by name");
        };
        assert!(error.to_string().contains("--ingress"), "{error}");
    }
}
