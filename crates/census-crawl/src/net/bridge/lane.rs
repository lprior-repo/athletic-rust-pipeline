use super::wire::{BrowserOutcome, RequestSpec};
use crate::ingress::Ingress;
use crate::net::FetchError;
use restate_sdk::ingress::{ClientError, RequestTarget};
use restate_sdk::prelude::Json;
use url::Url;

const FETCH_HANDLER: &str = "fetch";

#[derive(Clone)]
pub struct BrowserLane {
    client: Ingress,
}

impl BrowserLane {
    pub fn over(client: Ingress) -> Self {
        Self { client }
    }

    pub(in crate::net) async fn answer(
        &self,
        spec: &RequestSpec,
    ) -> Result<BrowserOutcome, FetchError> {
        let parsed = Url::parse(&spec.url).map_err(|source| FetchError::Policy {
            detail: format!("cannot parse browser URL: {source}"),
        })?;
        admitted(&parsed)?;
        let response = self
            .client
            .request::<Json<RequestSpec>, Json<BrowserOutcome>>(
                RequestTarget::object(super::SESSION_OBJECT, super::SESSION_KEY, FETCH_HANDLER),
                Json(spec.clone()),
            )
            .call()
            .await
            .map_err(|error| lane_error(spec, &error))?;
        response
            .into_body()
            .map(|Json(outcome)| outcome)
            .map_err(|error| lane_error(spec, &error))
    }
}

fn lane_error(spec: &RequestSpec, error: &ClientError) -> FetchError {
    FetchError::BrowserLane {
        url: spec.semantic_url.clone(),
        detail: ingress_detail(error),
        retryable: false,
    }
}

fn ingress_detail(error: &ClientError) -> String {
    let detail = error.response().and_then(|response| {
        serde_json::from_slice::<IngressFailure>(response.body())
            .ok()
            .map(|IngressFailure { message }| (response.status(), message))
    });
    match detail {
        Some((status, message)) => format!("ingress returned HTTP status {status}: {message}"),
        None => error.to_string(),
    }
}

#[derive(serde::Deserialize)]
struct IngressFailure {
    message: String,
}

pub fn validate_origin(url: &str) -> Result<(), FetchError> {
    let parsed = url::Url::parse(url).map_err(|source| FetchError::Policy {
        detail: format!("cannot parse browser URL: {source}"),
    })?;
    admitted(&parsed)
}

fn admitted(parsed: &Url) -> Result<(), FetchError> {
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::Policy {
            detail: format!(
                "scheme {scheme} is not admitted to the browser lane; only http and https are"
            ),
        });
    }
    let host = parsed
        .host_str()
        .map_or(Default::default(), core::convert::identity);
    if super::ADMITTED_BROWSER_ORIGINS.contains(&host) {
        Ok(())
    } else {
        Err(FetchError::Policy {
            detail: format!(
                "host {host} not admitted to browser lane; allowed: {}",
                super::ADMITTED_BROWSER_ORIGINS.join(", ")
            ),
        })
    }
}
