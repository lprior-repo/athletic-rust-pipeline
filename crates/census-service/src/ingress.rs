use anyhow::{bail, Result};
use census_crawl::ingress::Ingress;
use restate_sdk::ingress::ClientError;
use std::time::Duration;
use url::{Host, Url};

pub const DEFAULT_ORIGIN: &str = "http://127.0.0.1:18095/";

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

pub fn client(origin: &str) -> Result<Ingress> {
    build(origin, READ_TIMEOUT)
}

pub fn job_client(origin: &str) -> Result<Ingress> {
    build(origin, JOB_TIMEOUT)
}

fn build(origin: &str, timeout: Duration) -> Result<Ingress> {
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
    Ok(census_crawl::ingress::client(
        url.as_str().parse()?,
        http_client(timeout)?,
    )?)
}

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
