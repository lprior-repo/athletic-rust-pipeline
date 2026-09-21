//! The fetch loop: cache lookup, robots check, per-host pacing, timeout, retry and dispatch.

use super::cache::{read_cache, write_cache};
use super::client::HostState;
use super::decode::process_response;
use super::request::{build_request, jittered_delay, RequestBody};
use super::{
    now_iso8601, FetchError, FetchOptions, FetchOutcome, Fetcher, MAX_RETRIES, MIN_AUTHORIZED_DELAY,
};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, warn};

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
            let state = hosts
                .get_mut(host)
                .ok_or_else(|| anyhow::anyhow!("host {host} not registered in host_gate"));
            match state {
                Ok(s) => {
                    let now = std::time::Instant::now();
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
                Err(e) => {
                    warn!("host not registered: {e}");
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
    ) -> Result<FetchOutcome> {
        let extra = body
            .as_ref()
            .map(|(key, _)| key.clone())
            .unwrap_or_default();
        let key = Self::key_for(method, url, &extra);
        let (body_path, meta_path) = self.cache_paths(&key);
        let cached = read_cache(&body_path, &meta_path)?;

        if let Some(meta) = cached.as_ref() {
            if !options.refresh {
                if let Ok(bytes) = std::fs::read(&body_path) {
                    if meta.status == 200 || (options.allow_not_found && meta.status == 404) {
                        {
                            let mut stats = self.stats.lock().await;
                            stats.cache_hits = stats.cache_hits.saturating_add(1);
                        }
                        return Ok(FetchOutcome {
                            url: url.to_string(),
                            method: method.to_string(),
                            status: meta.status,
                            sha256: meta.sha256.clone(),
                            bytes: meta.bytes,
                            fetched_at: meta.fetched_at.clone(),
                            from_cache: true,
                            content_type: meta.content_type.clone(),
                            body: bytes,
                        });
                    }
                }
            }
        }

        let parsed = url::Url::parse(url).with_context(|| format!("invalid url {url}"))?;
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

        let rules = self.robots_for(&origin).await;
        if !rules.allows(&path_and_query) {
            if self.is_authorized_host(&host) {
                // Operator-authorized host: the rule is recorded on the run, not enforced.
                {
                    let mut stats = self.stats.lock().await;
                    stats.robots_authorized = stats.robots_authorized.saturating_add(1);
                }
                debug!(host = %host, path = %path_and_query, "robots rule overridden by host authorization");
            } else {
                let mut stats = self.stats.lock().await;
                stats.robots_blocked = stats.robots_blocked.saturating_add(1);
                return Err(FetchError::Robots(url.to_string()).into());
            }
        }
        let gate = self.host_gate(&host, rules.crawl_delay).await;

        // Retry with bounded exponential backoff + jitter.
        let mut last_err: Option<FetchError> = None;
        for attempt in 1..=MAX_RETRIES {
            let _permit = gate.lock().await;
            self.wait_turn(&host).await;

            // Build the HTTP request.
            let request = build_request(
                &self.client,
                method,
                url,
                body.as_ref().map(|(_, p)| p),
                &options.headers,
                cached.as_ref(),
                options.refresh,
            )?;

            // Execute with per-request timeout.
            let response = tokio::time::timeout(Duration::from_secs(timeout_secs), request.send())
                .await
                .map_err(|_| FetchError::Timeout {
                    url: url.to_string(),
                    timeout_secs,
                })?
                .map_err(|source| FetchError::Transport {
                    url: url.to_string(),
                    source,
                })?;

            let status = response.status().as_u16();

            // Count the request once. Bodies are counted inside `process_response`; every status
            // that never reaches it (304, 5xx, 429) is counted here instead.
            if status != 200 && status != 404 {
                let mut stats = self.stats.lock().await;
                stats.requests = stats.requests.saturating_add(1);
                let per_host = stats.per_host.entry(host.clone()).or_insert(0);
                *per_host = per_host.saturating_add(1);
            }

            match status {
                200 | 404 => {
                    // Process body.
                    let outcome = process_response(
                        response,
                        url,
                        method,
                        &host,
                        &body_path,
                        &meta_path,
                        options,
                        &mut *self.stats.lock().await,
                    )
                    .await?;
                    return Ok(outcome);
                }
                304 => {
                    // Conditional GET: use cached body, update timestamps.
                    if let Some(meta) = cached.as_ref() {
                        if let Ok(bytes) = std::fs::read(&body_path) {
                            let mut refreshed = meta.clone();
                            refreshed.fetched_at = now_iso8601();
                            write_cache(&body_path, &meta_path, &bytes, &refreshed)?;
                            {
                                let mut stats = self.stats.lock().await;
                                stats.conditional_304 = stats.conditional_304.saturating_add(1);
                            }
                            return Ok(FetchOutcome {
                                url: url.to_string(),
                                method: method.to_string(),
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
                    // Cache body disappeared — fall through to re-fetch.
                    if attempt < MAX_RETRIES {
                        last_err = Some(FetchError::Http {
                            status: 304,
                            url: url.to_string(),
                        });
                        let delay = jittered_delay(attempt);
                        debug!(
                            attempt,
                            delay_ms = delay.as_millis(),
                            "retrying after backoff"
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    let mut stats = self.stats.lock().await;
                    stats.errors = stats.errors.saturating_add(1);
                    return Err(FetchError::Http {
                        status: 304,
                        url: url.to_string(),
                    }
                    .into());
                }
                _ => {
                    // Non-200/404/304: record error, possibly retry on transport-like codes.
                    {
                        let mut stats = self.stats.lock().await;
                        stats.errors = stats.errors.saturating_add(1);
                    }
                    warn!(status, url, "non-success response");
                    let http_error = || FetchError::Http {
                        status,
                        url: url.to_string(),
                    };
                    // Retry on server errors (5xx) and client errors (429 rate-limit).
                    if status >= 500 || status == 429 {
                        if attempt < MAX_RETRIES {
                            last_err = Some(http_error());
                            let delay = jittered_delay(attempt);
                            debug!(
                                attempt,
                                status,
                                delay_ms = delay.as_millis(),
                                "retrying on server error"
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    } else if status == 404 && !options.allow_not_found {
                        // 404 is not retried — it's a terminal result.
                        return Err(http_error().into());
                    }
                    // Other errors (4xx except 429/404) are not retried.
                    last_err = Some(http_error());
                    break;
                }
            }
        }

        // All retries exhausted.
        let err = last_err.unwrap_or_else(|| FetchError::Timeout {
            url: url.to_string(),
            timeout_secs,
        });
        Err(err.into())
    }
}
