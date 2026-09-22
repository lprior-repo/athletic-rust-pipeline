//! One attempt of the fetch loop: the plan it runs from, and the verdict its response earns.

use crate::net::cache::{write_cache, CacheMeta};
use crate::net::decode::process_response;
use crate::net::request::{build_request, wait_backoff, RequestBody};
use crate::net::{now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher, MAX_RETRIES};
use census_domain::model::AccessBlockKind;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, warn};

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

/// What one attempt concluded.
enum Step {
    /// Publish this outcome.
    Publish(FetchOutcome),
    /// Record the error and take the next attempt.
    Retry(FetchError),
    /// Record the error and end the loop.
    Stop(FetchError),
    /// Fail the fetch with this error.
    Fail(FetchError),
}

impl Fetcher {
    /// Retry with bounded exponential backoff + jitter until an attempt publishes or the attempt
    /// budget runs out.
    pub(super) async fn retry_loop(
        &self,
        gate: Arc<Mutex<()>>,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let mut last_err: Option<FetchError> = None;
        for attempt in 1..=MAX_RETRIES {
            let _permit = gate.lock().await;
            self.wait_turn(plan.host).await;
            match self.attempt_once(plan, attempt).await? {
                Step::Publish(outcome) => return Ok(outcome),
                Step::Retry(error) => last_err = Some(error),
                Step::Stop(error) => {
                    last_err = Some(error);
                    break;
                }
                Step::Fail(error) => return Err(error),
            }
        }
        // All retries exhausted.
        let err = last_err.unwrap_or_else(|| FetchError::Timeout {
            url: plan.url.to_string(),
            timeout_secs: plan.timeout_secs,
        });
        Err(err)
    }

    /// One attempt: send the request, count it, record what the host said about access, and read the
    /// response's verdict.
    async fn attempt_once(&self, plan: &FetchPlan<'_>, attempt: u32) -> Result<Step, FetchError> {
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
                Ok(Step::Publish(outcome))
            }
            304 => self.replay_cached(plan, attempt).await,
            _ => Ok(self.status_verdict(status, plan, attempt).await),
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
    async fn count_request(&self, host: &str, status: u16) {
        if status == 200 || status == 404 {
            return;
        }
        let mut stats = self.stats.lock().await;
        stats.requests = stats.requests.saturating_add(1);
        let per_host = stats.per_host.entry(host.to_string()).or_insert(0);
        *per_host = per_host.saturating_add(1);
    }

    /// Conditional GET: publish the cached body with refreshed timestamps, or re-fetch.
    async fn replay_cached(&self, plan: &FetchPlan<'_>, attempt: u32) -> Result<Step, FetchError> {
        if let Some(meta) = plan.cached {
            if let Ok(bytes) = std::fs::read(plan.body_path) {
                let mut refreshed = meta.clone();
                refreshed.fetched_at = now_iso8601();
                write_cache(plan.body_path, plan.meta_path, &bytes, &refreshed)?;
                {
                    let mut stats = self.stats.lock().await;
                    stats.conditional_304 = stats.conditional_304.saturating_add(1);
                }
                return Ok(Step::Publish(FetchOutcome {
                    url: plan.url.to_string(),
                    method: plan.method.to_string(),
                    status: meta.status,
                    sha256: meta.sha256.clone(),
                    bytes: meta.bytes,
                    fetched_at: refreshed.fetched_at,
                    from_cache: false,
                    content_type: meta.content_type.clone(),
                    body: bytes,
                }));
            }
        }
        // The cache body disappeared — fall through to re-fetch.
        let error = FetchError::Http {
            status: 304,
            url: plan.url.to_string(),
        };
        if attempt < MAX_RETRIES {
            let delay = wait_backoff(attempt).await;
            debug!(
                attempt,
                delay_ms = delay.as_millis(),
                "retrying after backoff"
            );
            return Ok(Step::Retry(error));
        }
        let mut stats = self.stats.lock().await;
        stats.errors = stats.errors.saturating_add(1);
        Ok(Step::Fail(error))
    }

    /// Non-200/404/304: record the error, and back off on the statuses that are worth retrying.
    async fn status_verdict(&self, status: u16, plan: &FetchPlan<'_>, attempt: u32) -> Step {
        {
            let mut stats = self.stats.lock().await;
            stats.errors = stats.errors.saturating_add(1);
        }
        let url = plan.url;
        warn!(status, url, "non-success response");
        let error = FetchError::Http {
            status,
            url: plan.url.to_string(),
        };
        // Retry the statuses [`FetchError::retryable`] classifies as transient: server errors
        // (5xx) and client errors (429 rate-limit).
        if error.retryable() {
            if attempt < MAX_RETRIES {
                let delay = wait_backoff(attempt).await;
                debug!(
                    attempt,
                    status,
                    delay_ms = delay.as_millis(),
                    "retrying on server error"
                );
                return Step::Retry(error);
            }
        } else if status == 404 && !plan.options.allow_not_found {
            // 404 is not retried — it's a terminal result.
            return Step::Fail(error);
        }
        // Other errors (4xx except 429/404) are not retried.
        Step::Stop(error)
    }
}

/// The access condition one HTTP status states, when it states one.
///
/// `403` is the source refusing a path its own robots rules allow, and `429` is the source asking for
/// less traffic. Both are observations about the host, which is why they are recorded against it
/// rather than against the URL that happened to be in flight.
fn blocking_kind(status: u16) -> Option<AccessBlockKind> {
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
