pub(super) use super::attempt_helper::blocking_kind;
use super::attempt_helper::retry_after_secs;
use super::cache_writer::cache_and_record;
use crate::net::cache::{replay_cache, CacheMeta};
use crate::net::request::RequestBody;
use crate::net::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::OwnedMutexGuard;
use tracing::warn;

pub(super) struct FetchPlan<'a> {
    pub(super) method: &'a str,
    pub(super) url: &'a str,
    pub(super) payload: Option<&'a RequestBody>,
    pub(super) host: &'a str,
    pub(super) crawl_delay: Option<Duration>,
    pub(super) family: Option<String>,
    pub(super) body_path: &'a Path,
    pub(super) meta_path: &'a Path,
    pub(super) cached: Option<&'a CacheMeta>,
    pub(super) options: &'a FetchOptions,
    pub(super) representation: &'a crate::net::RepresentationHeaders,
    pub(super) timeout_secs: u64,
}

impl Fetcher {
    pub(super) async fn fetch_once(
        &self,
        plan: &FetchPlan<'_>,
    ) -> Result<FetchOutcome, FetchError> {
        let mut current_url = plan.url.to_string();
        let original_url = plan.url.to_string();
        let mut permit: Option<OwnedMutexGuard<()>> = None;
        for hop in 0..5 {
            let (host, origin) = super::request_target(&current_url)?;
            self.destination.validate_url(&current_url)?;
            self.origin_locks.ensure(&origin)?;
            if self.host_blocked(&host, &crate::net::now_iso8601()).await {
                return Err(FetchError::Policy {
                    detail: format!("host {host} is inside a recorded access cooldown"),
                });
            }
            let crawl_delay = match hop {
                0 => plan.crawl_delay,
                _ => self.robots_for(&origin, &host).await.crawl_delay,
            };
            let _hop_permit = if hop == 0 || self.family_of(&host) == plan.family {
                None
            } else {
                self.family_permit_now(&host).await?
            };
            let gate = self.host_gate(&host, crawl_delay).await;
            drop(permit.take());
            permit = Some(gate.lock_owned().await);
            self.wait_turn(&host).await;
            if self.host_blocked(&host, &crate::net::now_iso8601()).await {
                return Err(FetchError::Policy {
                    detail: format!(
                        "host {host} entered a recorded access cooldown before this request was dispatched"
                    ),
                });
            }
            let response = self.dispatch_at(&current_url, plan).await?;
            let status = response.status().as_u16();
            self.count_request(&host, status).await;
            match self.resolve_redirect_target(&current_url, &original_url, &response)? {
                Some(next_url) => current_url = next_url.to_string(),
                None => {
                    return self
                        .handle_final_response(plan, &current_url, &host, response)
                        .await
                }
            }
        }
        Err(FetchError::Policy {
            detail: format!("redirect limit exceeded after 5 hops for {original_url}"),
        })
    }

    async fn handle_final_response(
        &self,
        plan: &FetchPlan<'_>,
        current_url: &str,
        current_host: &str,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        let status = response.status().as_u16();
        if let Some(kind) = blocking_kind(status) {
            self.record_access_condition(
                current_host,
                kind,
                status,
                retry_after_secs(&response),
                current_url.to_string(),
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

    fn resolve_redirect_target(
        &self,
        current_url: &str,
        original_url: &str,
        response: &reqwest::Response,
    ) -> Result<Option<url::Url>, FetchError> {
        if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
            return Ok(None);
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| FetchError::Policy {
                detail: format!("redirect response at {current_url} has no usable Location header"),
            })?;
        let next = redirect_target(current_url, location)?;
        let original = url::Url::parse(original_url).map_err(|source| FetchError::InvalidUrl {
            url: original_url.to_string(),
            source,
        })?;
        if !self.destination.permits_redirect(&original, &next) {
            return Err(FetchError::Policy {
                detail: format!("redirect from {current_url} to {next} bypasses admission"),
            });
        }
        if next
            .host_str()
            .and_then(crate::registry::transport_for_host)
            == Some(crate::registry::TransportKind::Browser)
        {
            return Err(FetchError::Policy {
                detail: format!("HTTP redirect to browser-transport destination {next}"),
            });
        }
        Ok(Some(next))
    }

    async fn process_ok(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        cache_and_record(response, plan, 200, &self.stats).await
    }

    async fn handle_404(
        &self,
        plan: &FetchPlan<'_>,
        response: reqwest::Response,
    ) -> Result<FetchOutcome, FetchError> {
        let status = 404u16;
        let outcome = cache_and_record(response, plan, status, &self.stats).await?;
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
        let elapsed_ms =
            u64::try_from(started.elapsed().as_millis()).map_or(u64::MAX, |value| value);
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
            if let Some(bytes) = replay_cache(plan.body_path, plan.meta_path, meta)? {
                {
                    let mut stats = self.stats.lock().await;
                    stats.conditional_304 = stats.conditional_304.saturating_add(1);
                }
                return Ok(FetchOutcome {
                    url: plan.url.to_string(),
                    response_url: meta.response_url.clone(),
                    method: plan.method.to_string(),
                    status: meta.status,
                    content_digest: meta.content_digest.clone(),
                    bytes: meta.bytes,
                    fetched_at: meta.fetched_at.clone(),
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

fn redirect_target(current_url: &str, location: &str) -> Result<url::Url, FetchError> {
    url::Url::parse(current_url)
        .and_then(|base| base.join(location))
        .map_err(|source| FetchError::InvalidUrl {
            url: current_url.to_string(),
            source,
        })
}
