//! The lane's client: one spec posted to the pipeline's `BrowserSession` object, and the transport's
//! whole answer read back.
//!
//! Exactly one process owns the headed profile, and it is not this one. The pipeline registers
//! `BrowserSession` on the Restate node this deployment talks to, so the census posts a
//! [`RequestSpec`] to that object's `fetch` handler instead of opening a browser of its own: the
//! profile keeps one owner, and the census keeps writing evidence in the shape it already has.
//!
//! Nothing here retries. The transport attempts exactly once and the durable layer owns retries
//! (ADR-002), so a verdict that says "another invocation is worth making" leaves as an error for that
//! layer to replay, and a refusal leaves with the access-condition row its caller recorded (§69).
//! The verdict is never re-derived here: it travels with the answer, and this module's only
//! classification is of the *call* itself — no node, no service, a wedged request.

use super::wire::{BrowserOutcome, RequestSpec};
use crate::net::FetchError;
use restate_sdk::ingress::{ClientError, RequestTarget, ReqwestClient};
use restate_sdk::prelude::Json;
use url::Url;

/// The handler that takes a [`RequestSpec`] and answers with a [`BrowserOutcome`].
const FETCH_HANDLER: &str = "fetch";

/// The census's client for the one headed profile.
#[derive(Clone)]
pub struct BrowserLane {
    client: ReqwestClient,
}

impl BrowserLane {
    /// Build a lane over an ingress client the deployment already validated.
    ///
    /// The client arrives built because the ingress origin is a deployment fact: the CLI's `ingress`
    /// module owns "which node, validated how, with what budget", and every command that talks to the
    /// pipeline builds its client there.
    pub fn over(client: ReqwestClient) -> Self {
        Self { client }
    }

    /// Post one spec and read the transport's whole answer.
    ///
    /// The answer is handed on unchanged, including the failure verdict. What is classified here is
    /// only the call's own transport failure — an ingress that is not running, a service the
    /// deployment does not have, a reply that is not this wire's JSON — and that is exactly the
    /// "applicable but cannot run" case whose row the caller records.
    ///
    /// A URL whose origin is not in [`super::ADMITTED_BROWSER_ORIGINS`] is refused before it
    /// leaves this process with a [`FetchError::Policy`] error.
    pub(in crate::net) async fn answer(
        &self,
        spec: &RequestSpec,
    ) -> Result<BrowserOutcome, FetchError> {
        let parsed = Url::parse(&spec.url).map_err(|source| FetchError::Policy {
            detail: format!("cannot parse browser URL: {source}"),
        })?;
        let host = parsed.host_str().unwrap_or_default();
        if !super::ADMITTED_BROWSER_ORIGINS.contains(&host) {
            return Err(FetchError::Policy {
                detail: format!(
                    "host {host} not admitted to browser lane; allowed: {}",
                    super::ADMITTED_BROWSER_ORIGINS.join(", ")
                ),
            });
        }
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

/// The lane's own failure: the call never reached a verdict.
///
/// Both halves of the call fail this way — the invocation itself (no ingress, no service) and the
/// reply's reading (a body that is not this wire's JSON) — and neither is a verdict about the
/// source, which is why neither is graded retryable here: the durable layer replays what the
/// transport classified, and a deployment that is missing its lane is not repaired by a retry.
fn lane_error(spec: &RequestSpec, error: &ClientError) -> FetchError {
    FetchError::BrowserLane {
        url: spec.semantic_url.clone(),
        detail: ingress_detail(error),
        retryable: false,
    }
}

/// Restate's own message for a refused call, when it sent one.
///
/// A terminal handler failure arrives as a JSON body, and that body is the only place the handler's
/// own words are: `ClientError`'s display says which status came back, never what the service said.
/// `cli::ingress` reads the same envelope for the census's own clients; the reading is repeated here
/// because the layering runs one way — the CLI knows this layer, this layer cannot reach the CLI —
/// and a lane that reported a bare `404` would not tell an operator that the pipeline's
/// `BrowserSession` service is missing from the deployment.
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

/// The `{"code":…,"message":…}` envelope Restate answers a terminal failure with.
#[derive(serde::Deserialize)]
struct IngressFailure {
    message: String,
}

///
/// This is the guard that prevents a request from being sent to an origin the browser session
/// does not serve.  Extracted from [`BrowserLane::answer`] so it can be exercised in tests
/// without constructing a [`restate_sdk::ingress::ReqwestClient`].
pub fn validate_origin(url: &str) -> Result<(), FetchError> {
    let parsed = url::Url::parse(url).map_err(|source| FetchError::Policy {
        detail: format!("cannot parse browser URL: {source}"),
    })?;
    let host = parsed.host_str().unwrap_or_default();
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
