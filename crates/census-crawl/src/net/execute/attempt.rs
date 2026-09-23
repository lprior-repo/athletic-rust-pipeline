//! One attempt of the fetch loop: the plan it runs from, and the verdict its response earns.

use crate::net::cache::{write_cache, CacheMeta};
use crate::net::decode::process_response;
use crate::net::request::{build_request, RequestBody};
use crate::net::{now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_domain::model::AccessBlockKind;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

/// Everything an attempt needs: the request's coordinates plus the cache paths it reads and writes.
pub(super) struct FetchPlan<'a> {
    pub(super) method: &'a str,
    pub(super) url: &'a str,
    pub(super) payload: Option<&'a RequestBody>,
    pub(super) host: &'a str,
    pub(super) body_path: &'a Path,
    pub(super) meta_path: &'a Path,
    pub(super) cached: Option<&'a CacheMeta>,
    pub(super) options: &'a FetchOptions,
    pub(super) timeout_secs: u64,
}

impl Fetcher {
    /// Send one request. The durable layer owns retries (ADR-002): a failure returns an error, and
    /// Restate replays the step under a policy the journal can account for. An in-process retry loop
    /// here would spend a second budget no operator can see, and the backoff would be lost on any
    /// restart — so the transport attempts exactly once, as the ADR's Decision states.
    pub(super) async fn fetch_once(
        &self,
        gate: Arc<Mutex<()>>,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let _permit = gate.lock().await;
        self.wait_turn(plan.host).await;
        self.attempt_once(plan).await
    }

    /// One attempt: send the request, count it, record what the host said about access, and read the
    /// response's verdict.
    async fn attempt_once(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
        let response = self.dispatch(plan).await?;
        let status = response.status().as_u16();
        self.count_request(plan.host, status).await;
        // A 403 or 429 is an observation about the *host*, not about this URL: it is recorded once
        // per run so a lane can stop on it instead of re-discovering it request by request.
        if let Some(kind) = blocking_kind(status) {
            self.record_access_condition(
                plan.host,
                kind,
                status,
                retry_after_secs(&response),
                plan.url.to_string(),
            )
            .await;
        }
        match status {
            200 | 404 => {
                // Process body.
                let outcome = process_response(
                    response,
                    plan.url,
                    plan.method,
                    plan.host,
                    plan.body_path,
                    plan.meta_path,
                    plan.options,
                    &mut *self.stats.lock().await,
                )
                .await?;
                Ok(outcome)
            }
            304 => self.replay_cached(plan).await,
            _ => Err(self.status_error(status, plan).await),
        }
    }

    /// Build the HTTP request and send it under the per-request timeout.
    async fn dispatch(&self, plan: &FetchPlan<'_>) -> Result<reqwest::Response, FetchError> {
        let request = build_request(
            &self.client,
            plan.method,
            plan.url,
            plan.payload,
            &plan.options.headers,
            plan.cached,
            plan.options.refresh,
        )?;
        let response = tokio::time::timeout(Duration::from_secs(plan.timeout_secs), request.send())
            .await
            .map_err(|_| FetchError::Timeout {
                url: plan.url.to_string(),
                timeout_secs: plan.timeout_secs,
            })?
            .map_err(|source| FetchError::Transport {
                url: plan.url.to_string(),
                source,
            })?;
        Ok(response)
    }

    /// Count the request once. Bodies are counted inside `process_response`; every status that
    /// never reaches it (304, 5xx, 429) is counted here instead.
    ///
    /// The browser lane's seat counts through the same call: one request, counted once, whichever
    /// transport carried it.
    pub(super) async fn count_request(&self, host: &str, status: u16) {
        if status == 200 || status == 404 {
            return;
        }
        let mut stats = self.stats.lock().await;
        stats.requests = stats.requests.saturating_add(1);
        let per_host = stats.per_host.entry(host.to_string()).or_insert(0);
        *per_host = per_host.saturating_add(1);
    }

    /// Conditional GET: publish the cached body with refreshed timestamps.
    ///
    /// A 304 whose body has vanished from disk is an error, not a re-fetch: there is nothing to
    /// publish, and whether to send the request again is the durable policy's call, not this
    /// function's.
    async fn replay_cached(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
        if let Some(meta) = plan.cached {
            if let Ok(bytes) = std::fs::read(plan.body_path) {
                let mut refreshed = meta.clone();
                refreshed.fetched_at = now_iso8601();
                write_cache(plan.body_path, plan.meta_path, &bytes, &refreshed)?;
                {
                    let mut stats = self.stats.lock().await;
                    stats.conditional_304 = stats.conditional_304.saturating_add(1);
                }
                return Ok(FetchOutcome {
                    url: plan.url.to_string(),
                    method: plan.method.to_string(),
                    status: meta.status,
                    sha256: meta.sha256.clone(),
                    bytes: meta.bytes,
                    fetched_at: refreshed.fetched_at,
                    from_cache: false,
                    content_type: meta.content_type.clone(),
                    body: bytes,
                });
            }
        }
        // The cache body disappeared: there is nothing to publish, so the fetch fails and the
        // durable layer decides whether to send it again.
        let mut stats = self.stats.lock().await;
        stats.errors = stats.errors.saturating_add(1);
        Err(FetchError::Http {
            status: 304,
            url: plan.url.to_string(),
        })
    }

    /// Non-200/404/304: record the error and return it for the durable layer to classify.
    ///
    /// The browser lane's seat reaches this for exactly the same statuses, so a refusal that
    /// arrives inside a capture (403, 5xx) is graded once, by this policy, whichever transport
    /// carried it.
    pub(super) async fn status_error(&self, status: u16, plan: &FetchPlan<'_>) -> FetchError {
        {
            let mut stats = self.stats.lock().await;
            stats.errors = stats.errors.saturating_add(1);
        }
        let url = plan.url;
        warn!(status, url, "non-success response");
        // Every status here is either a host observation (403/429, already recorded against the host)
        // or a fault the durable retry policy exists to absorb. The transport's job is to report it;
        // grading it as retryable or terminal belonged to the loop that no longer runs here.
        FetchError::Http {
            status,
            url: plan.url.to_string(),
        }
    }
}

/// The access condition one HTTP status states, when it states one.
///
/// `403` is the source refusing a path its own robots rules allow, and `429` is the source asking for
/// less traffic. Both are observations about the host, which is why they are recorded against it
/// rather than against the URL that happened to be in flight.
///
/// A capture the browser lane carries states the same two the same way: the status travels on the
/// capture, so the reading is shared rather than restated.
pub(super) fn blocking_kind(status: u16) -> Option<AccessBlockKind> {
    match status {
        403 => Some(AccessBlockKind::Forbidden),
        429 => Some(AccessBlockKind::RateLimited),
        _ => None,
    }
}

/// The `Retry-After` one response published, when it published one in the delta-seconds form.
///
/// The HTTP-date form is deliberately not parsed: no source in this corpus has used it, and a wrong
/// guess at an instant is worse than no instant, so an unparsed value stays `None` and the policy
/// default applies.
fn retry_after_secs(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
}
