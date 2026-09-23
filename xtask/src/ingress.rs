//! The Restate ingress transport the census subcommands share.
//!
//! Same rules as the census binary's own transport (`crates/census-service/src/cli/ingress.rs`): the
//! origin must be a loopback HTTP origin without credentials, a path or a query, redirects and
//! proxies are off, and a terminal ingress failure is reported with Restate's own message — that
//! message is what tells an operator which flag to change.
//!
//! It is a second copy rather than a shared module because the two crates are separate compilation
//! units: `xtask` drives the deployment, and the census crate's transport is private to its command
//! line.

use anyhow::{bail, Result};
use restate_sdk::ingress::{ClientError, RequestTarget, ReqwestClient};
use std::future::Future;
use std::time::Duration;
use url::{Host, Url};

/// The project node's ingress origin: the deployment `census-serve` registers with.
pub const NODE_ORIGIN: &str = "http://127.0.0.1:18095/";

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

/// The budget for a call that only reads: a status, or the deployment's last produced report.
///
/// A read that has not answered in this long is wedged, and the short budget is what makes that
/// visible instead of silence.
const READ_TIMEOUT: Duration = Duration::from_secs(120);

/// The budget for a call that *runs* a job in the service: the append-log consolidation, the census
/// report, the best-mark reduction and the workbook each hold the store for minutes at full size. The
/// client must not abandon a call the deployment is still executing — the invocation keeps running
/// either way, and a client that gave up cannot report what it produced.
const JOB_TIMEOUT: Duration = Duration::from_secs(900);

/// Build the ingress client for `origin` with the read budget ([`READ_TIMEOUT`]), and hand back the
/// origin in its URL form.
///
/// The caller prints the URL form: the invocation line has to name the request the client will make
/// (`http://127.0.0.1:18095/Census/status`), and the string an operator typed may have no trailing
/// slash to join the handler path onto.
pub fn client(origin: &str) -> Result<(String, ReqwestClient)> {
    build(origin, READ_TIMEOUT)
}

/// Build the ingress client for `origin` with the job budget ([`JOB_TIMEOUT`]).
///
/// Only the subcommands that submit a run which owns the store's writer for minutes use this: a read
/// is answered in milliseconds, and a long budget there would hide a wedged connection.
pub fn job_client(origin: &str) -> Result<(String, ReqwestClient)> {
    build(origin, JOB_TIMEOUT)
}

/// Validate `origin` and build its client under `timeout`.
///
/// A non-loopback origin, a path, credentials or a query is refused before any work is asked for: the
/// census deployment runs beside its store on this machine, and a mistyped origin must fail loudly
/// instead of driving some other deployment.
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

/// Print the invocation line for one `call`, in the same `+ <line>` form [`crate::cmd::Cmd`] prints
/// for a child process.
///
/// The path is the one the client submits to — `/restate/call/<Service>/<handler>` under the origin,
/// which is the spelling Restate documents and [`restate_sdk::ingress::Client`] builds — so a run
/// transcript names the request that was actually made rather than a lookalike spelling the node
/// happens to accept.
pub fn announce(endpoint: &str, service: &str, handler: &str) {
    println!(
        "+ POST {endpoint}restate/call/{}",
        RequestTarget::service(service, handler)
    );
}

/// Run one ingress exchange on a current-thread runtime.
///
/// Every other command line this harness runs is a `std::process::Command`, so the runtime is built
/// here for the one exchange instead of turning `main` — and with it every other subcommand — async.
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
