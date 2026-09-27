pub(super) use super::attempt_helper::blocking_kind;
use super::attempt_helper::retry_after_secs;
use super::cache_writer::cache_and_record;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::request::RequestBody;
use crate::net::{now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::warn;

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
    pub(super) async fn fetch_once(
        &self,
        gate: Arc<Mutex<()>>,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let _permit = gate.lock().await;
        self.wait_turn(plan.host).await;
        self.attempt_once(plan).await
    }

    async fn attempt_once(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
        let response = self.dispatch(plan).await?;
        let status = response.status().as_u16();
        self.count_request(plan.host, status).await;
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

    pub(super) async fn count_request(&self, host: &str, status: u16) {
        if status == 200 || status == 404 {
            return;
        }
        let mut stats = self.stats.lock().await;
        stats.requests = stats.requests.saturating_add(1);
        let entry = stats.per_host.entry(host.to_string()).or_default();
        entry.requests = entry.requests.saturating_add(1);
    }

    pub(super) async fn record_transport(
        &self,
        started: Instant,
        outcome: &Result<FetchOutcome, FetchError>,
    ) {
        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut stats = self.stats.lock().await;
        stats.record_latency(elapsed_ms);
        match outcome {
            Err(FetchError::RateLimited { .. } | FetchError::Http { status: 429, .. }) => {
                stats.rate_limited = stats.rate_limited.saturating_add(1);
            }
            Err(FetchError::Timeout { .. }) => {
                stats.timeouts = stats.timeouts.saturating_add(1);
            }
            _ => {}
        }
    }

    async fn replay_cached(&self, plan: &FetchPlan<'_>) -> Result<FetchOutcome, FetchError> {
        if let Some(meta) = plan.cached {
            if let Ok(bytes) = std::fs::read(plan.body_path) {
                if bytes.len() != meta.bytes || content_digest(&bytes) != meta.content_digest {
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
                    content_digest: meta.content_digest.clone(),
                    bytes: meta.bytes,
                    fetched_at: refreshed.fetched_at,
                    from_cache: false,
                    content_type: meta.content_type.clone(),
                    body: bytes,
                });
            }
        }
        let mut stats = self.stats.lock().await;
        stats.errors = stats.errors.saturating_add(1);
        Err(FetchError::Http {
            status: 304,
            url: plan.url.to_string(),
        })
    }
}
