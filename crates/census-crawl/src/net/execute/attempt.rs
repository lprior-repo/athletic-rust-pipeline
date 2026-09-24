//! One attempt of the fetch loop: the plan it runs from, and the verdict its response earns.

pub(super) use super::attempt_helper::blocking_kind;
use super::attempt_helper::retry_after_secs;
use super::cache_writer::cache_and_record;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::request::RequestBody;
use crate::net::{now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::path::Path;
use std::sync::Arc;
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
            200 => self.process_ok(plan, response).await,
            404 => self.handle_404(plan, response).await,
            304 => self.replay_cached(plan).await,
            _ => Err(self.status_error(status, plan).await),
        }
    }

    /// Process a 200 response: read body, cache, update stats, return outcome.
    async fn process_ok(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        cache_and_record(
            response,
            plan.url,
            plan.method,
            200,
            plan.body_path,
            plan.meta_path,
            &self.stats,
        )
        .await
    }

    /// Handle a 404: cache the evidence, then return `Ok` or `Err` depending on `allow_not_found`.
    ///
    /// The body and metadata are always cached — a 404 is a real answer the run should remember.
    /// When `allow_not_found` is `true`, the caller expects the 404 as a normal outcome.
    /// When `false`, the caller wants a 404 to propagate as an error.
    async fn handle_404(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        let status = 404u16;
        let outcome = cache_and_record(
            response,
            plan.url,
            plan.method,
            status,
            plan.body_path,
            plan.meta_path,
            &self.stats,
        )
        .await?;
        if plan.options.allow_not_found {
            Ok(outcome)
        } else {
            {
                let mut stats = self.stats.lock().await;
                stats.errors = stats.errors.saturating_add(1);
                warn!(status, url = plan.url, "non-success response");
            }
            Err(FetchError::Http {
                status,
                url: plan.url.to_string(),
            })
        }
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
                // Verify the cached body before reusing it.
                if bytes.len() != meta.bytes || content_digest(&bytes) != meta.content_digest {
                    // Body is corrupted — fall through to error.
                    let mut stats = self.stats.lock().await;
                    stats.errors = stats.errors.saturating_add(1);
                    return Err(FetchError::Http {
                        status: 304,
                        url: plan.url.to_string(),
                    });
                }
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
                    sha256: meta.key_prefix.clone(),
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
}
