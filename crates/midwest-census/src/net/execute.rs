//! The fetch loop: cache lookup, robots check, per-host pacing, timeout, retry and dispatch.

use super::cache::{read_cache, CacheMeta};
use super::client::HostState;
use super::request::RequestBody;
use super::{FetchError, FetchOptions, FetchOutcome, Fetcher, MIN_AUTHORIZED_DELAY};
use crate::clock::{Clock, SystemClock};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, warn};

mod attempt;

use attempt::FetchPlan;

impl Fetcher {
    /// Serialize per host and enforce the configured (or robots-requested) spacing.
    async fn host_gate(&self, host: &str, robots_delay: Option<Duration>) -> Arc<Mutex<()>> {
        let configured = self
            .host_delays
            .get(host)
            .copied()
            .unwrap_or(self.default_delay);
        let mut effective = match robots_delay {
            Some(robots) if robots > configured => robots,
            _ => configured,
        };
        // Authorization permits a host whose robots rules would otherwise block the request; it
        // never permits a faster rate than the collection policy's 2 rps ceiling.
        if self.is_authorized_host(host) && effective < MIN_AUTHORIZED_DELAY {
            effective = MIN_AUTHORIZED_DELAY;
        }
        let mut hosts = self.hosts.lock().await;
        let state = hosts.entry(host.to_string()).or_insert_with(|| HostState {
            gate: Arc::new(Mutex::new(())),
            next_allowed: None,
            delay: effective,
        });
        state.delay = state.delay.max(effective);
        state.gate.clone()
    }

    async fn wait_turn(&self, host: &str) {
        let wait = {
            let mut hosts = self.hosts.lock().await;
            match hosts.get_mut(host) {
                Some(s) => {
                    let now = SystemClock.now();
                    // The reserved slot is a floor: resume from the later of "now" and the slot,
                    // then push the slot one delay further. `checked_add` keeps the instant
                    // arithmetic panic-free, and a clock far enough out to overflow `Instant`
                    // cannot occur in a run, so "no reservation" is the honest answer there.
                    let from = match s.next_allowed {
                        Some(at) if at > now => at,
                        _ => now,
                    };
                    let wait = from.saturating_duration_since(now);
                    s.next_allowed = from.checked_add(s.delay);
                    wait
                }
                None => {
                    warn!("host {host} not registered in host_gate");
                    Duration::ZERO
                }
            }
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
    /// Core fetch logic with caching, rate limiting, retry, and timeout.
    pub(super) async fn fetch(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
        timeout_secs: u64,
    ) -> Result<FetchOutcome, FetchError> {
        let extra = body
            .as_ref()
            .map(|(key, _)| key.clone())
            .unwrap_or_default();
        let key = Self::key_for(method, url, &extra);
        let (body_path, meta_path) = self.cache_paths(&key);
        let cached = read_cache(&body_path, &meta_path)?;
        if let Some(meta) = cached.as_ref() {
            if let Some(outcome) = self
                .cached_outcome(method, url, &body_path, meta, options)
                .await?
            {
                return Ok(outcome);
            }
        }

        let (host, origin, path_and_query) = request_target(url)?;
        let crawl_delay = self
            .robots_gate(url, &host, &path_and_query, &origin)
            .await?;
        let gate = self.host_gate(&host, crawl_delay).await;
        let plan = FetchPlan {
            method,
            url,
            payload: body.as_ref().map(|(_, payload)| payload),
            host: &host,
            body_path: &body_path,
            meta_path: &meta_path,
            cached: cached.as_ref(),
            options,
            timeout_secs,
        };
        self.retry_loop(gate, &plan).await
    }

    /// Serve the request from the cache when a usable body is already on disk.
    async fn cached_outcome(
        &self,
        method: &str,
        url: &str,
        body_path: &Path,
        meta: &CacheMeta,
        options: &FetchOptions,
    ) -> Result<Option<FetchOutcome>, FetchError> {
        if options.refresh {
            return Ok(None);
        }
        let Ok(bytes) = std::fs::read(body_path) else {
            return Ok(None);
        };
        if meta.status != 200 && !(options.allow_not_found && meta.status == 404) {
            return Ok(None);
        }
        {
            let mut stats = self.stats.lock().await;
            stats.cache_hits = stats.cache_hits.saturating_add(1);
        }
        Ok(Some(FetchOutcome {
            url: url.to_string(),
            method: method.to_string(),
            status: meta.status,
            sha256: meta.sha256.clone(),
            bytes: meta.bytes,
            fetched_at: meta.fetched_at.clone(),
            from_cache: true,
            content_type: meta.content_type.clone(),
            body: bytes,
        }))
    }

    /// Fetch the origin's robots rules and apply them to one path.
    async fn robots_gate(
        &self,
        url: &str,
        host: &str,
        path_and_query: &str,
        origin: &str,
    ) -> Result<Option<Duration>, FetchError> {
        let rules = self.robots_for(origin).await;
        if rules.allows(path_and_query) {
            return Ok(rules.crawl_delay);
        }
        if self.is_authorized_host(host) {
            // Operator-authorized host: the rule is recorded on the run, not enforced.
            {
                let mut stats = self.stats.lock().await;
                stats.robots_authorized = stats.robots_authorized.saturating_add(1);
            }
            debug!(
                host = %host,
                path = %path_and_query,
                "robots rule overridden by host authorization"
            );
            return Ok(rules.crawl_delay);
        }
        let mut stats = self.stats.lock().await;
        stats.robots_blocked = stats.robots_blocked.saturating_add(1);
        Err(FetchError::Robots(url.to_string()))
    }
}

/// Split a URL into its host, its origin and the path-and-query robots rules match.
fn request_target(url: &str) -> Result<(String, String, String), FetchError> {
    let parsed = url::Url::parse(url).map_err(|source| FetchError::InvalidUrl {
        url: url.to_string(),
        source,
    })?;
    let host = parsed.host_str().unwrap_or_default().to_string();
    let origin = format!(
        "{}://{}",
        parsed.scheme(),
        parsed
            .port()
            .map(|p| format!("{host}:{p}"))
            .unwrap_or_else(|| host.clone())
    );
    let path_and_query = match parsed.query() {
        Some(query) => format!("{}?{}", parsed.path(), query),
        None => parsed.path().to_string(),
    };
    Ok((host, origin, path_and_query))
}
