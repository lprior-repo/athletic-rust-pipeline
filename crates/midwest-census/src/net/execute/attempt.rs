//! One attempt of the fetch loop: the plan it runs from, and the verdict its response earns.

use crate::net::cache::{write_cache, CacheMeta};
use crate::net::decode::process_response;
use crate::net::request::{build_request, wait_backoff, RequestBody};
use crate::net::{now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher, MAX_RETRIES};
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
            let response = self.dispatch(plan).await?;
            let status = response.status().as_u16();
            self.count_request(plan.host, status).await;
            let step = match status {
                200 | 404 => {
                    // Process body.
                    Step::Publish(
                        process_response(
                            response,
                            plan.url,
                            plan.method,
                            plan.host,
                            plan.body_path,
                            plan.meta_path,
                            plan.options,
                            &mut *self.stats.lock().await,
                        )
                        .await?,
                    )
                }
                304 => self.replay_cached(plan, attempt).await?,
                _ => self.status_verdict(status, plan, attempt).await,
            };
            match step {
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
