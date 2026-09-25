//! The fetch loop: cache lookup, robots check, per-host pacing, timeout, retry and dispatch.

use super::cache::{read_cache, CacheMeta};
use super::client::HostState;
use super::request::RequestBody;
use super::{FetchError, FetchOptions, FetchOutcome, Fetcher, MIN_AUTHORIZED_DELAY};
use census_domain::model::AccessBlockKind;
use census_store::clock::{Clock, SystemClock};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

mod attempt;
mod attempt_helper;
mod body_reader;
mod browser;
mod cache_writer;

use attempt::FetchPlan;

/// Which budget map a request is charged to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Family,
    Host,
}

/// The budget a request is charged to: one shared budget for a whole source family (§10), or the
/// host's own when the run configured no family for it.
#[derive(Clone, PartialEq, Eq)]
enum PaceScope {
    Family(String),
    Host(String),
}

impl PaceScope {
    fn kind(&self) -> ScopeKind {
        match self {
            PaceScope::Family(_) => ScopeKind::Family,
            PaceScope::Host(_) => ScopeKind::Host,
        }
    }

    fn into_key(self) -> String {
        match self {
            PaceScope::Family(key) | PaceScope::Host(key) => key,
        }
    }
}

impl Fetcher {
    /// The budget one request is charged to: a shared source family when the run configured one for
    /// this host (§10), otherwise the host's own.
    fn pace_scope(&self, host: &str) -> PaceScope {
        match self.family_of(host) {
            Some(family) => PaceScope::Family(family),
            None => PaceScope::Host(host.trim().to_ascii_lowercase()),
        }
    }

    /// The spacing this request must observe: the family's when it is charged to one, else the
    /// host's own entry, else the run's default.
    fn configured_delay(&self, scope: &PaceScope) -> Duration {
        match scope {
            PaceScope::Family(family) => self.family_delay(family).unwrap_or(self.default_delay),
            PaceScope::Host(host) => self
                .host_delays
                .get(host)
                .copied()
                .unwrap_or(self.default_delay),
        }
    }

    /// Serialize per budget and enforce the configured (or robots-requested) spacing.
    async fn host_gate(&self, host: &str, robots_delay: Option<Duration>) -> Arc<Mutex<()>> {
        let scope = self.pace_scope(host);
        let configured = self.configured_delay(&scope);
        let mut effective = match robots_delay {
            Some(robots) if robots > configured => robots,
            _ => configured,
        };
        // Authorization permits a host whose robots rules would otherwise block the request; it
        // never permits a faster rate than the collection policy's 2 rps ceiling.
        if self.is_authorized_host(host) && effective < MIN_AUTHORIZED_DELAY {
            effective = MIN_AUTHORIZED_DELAY;
        }
        let kind = scope.kind();
        let key = scope.into_key();
        let mut buckets = match kind {
            ScopeKind::Family => self.families.lock().await,
            ScopeKind::Host => self.hosts.lock().await,
        };
        let state = buckets.entry(key).or_insert_with(|| HostState {
            gate: Arc::new(Mutex::new(())),
            next_allowed: None,
            delay: effective,
        });
        state.delay = state.delay.max(effective);
        state.gate.clone()
    }

    async fn wait_turn(&self, host: &str) {
        let scope = self.pace_scope(host);
        let kind = scope.kind();
        let key = scope.into_key();
        let wait = {
            let mut buckets = match kind {
                ScopeKind::Family => self.families.lock().await,
                ScopeKind::Host => self.hosts.lock().await,
            };
            match buckets.get_mut(&key) {
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
    /// Core fetch logic: serve from cache, take the host's turn, then attempt once inside the timeout.
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
        // read_cache verifies the body against the metadata; a mismatch returns None (miss).
        let cached = read_cache(&body_path, &meta_path)?;
        if let Some((meta, body)) = cached.as_ref() {
            if let Some(outcome) = self
                .cached_outcome(method, url, meta, options, body.clone())
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
            cached: cached.as_ref().map(|(meta, _)| meta),
            options,
            timeout_secs,
        };
        // Which transport carries the request is the registry's declaration, not the caller's
        // request: a registered host whose table entry says `Browser` is the one place that fact
        // lives, and a host no descriptor claims keeps the HTTP path it has always had.
        let started = Instant::now();
        let outcome = match crate::registry::transport_for_host(&host) {
            Some(crate::registry::TransportKind::Browser) => self.fetch_browser(gate, &plan).await,
            _ => self.fetch_once(gate, &plan).await,
        };
        self.record_transport(started, &outcome).await;
        outcome
    }

    /// Serve the request from the cache when a verified body is already on disk.
    async fn cached_outcome(
        &self,
        method: &str,
        url: &str,
        meta: &CacheMeta,
        options: &FetchOptions,
        body: Vec<u8>,
    ) -> Result<Option<FetchOutcome>, FetchError> {
        if options.refresh {
            return Ok(None);
        }
        if meta.status != 200 && !(options.allow_not_found && meta.status == 404) {
            return Ok(None);
        }
        {
            let mut stats = self.stats.lock().await;
            stats.cache_hits = stats.cache_hits.saturating_add(1);
            // The origin itself saw nothing, but the request still names it: §45's per-provider row
            // banks this as a cache hit, so `requests - cache_hits` stays the physical traffic that
            // host actually answered.
            let entry = stats.per_host.entry(crate::net::host_of(url)).or_default();
            entry.requests = entry.requests.saturating_add(1);
            entry.cache_hits = entry.cache_hits.saturating_add(1);
        }
        Ok(Some(FetchOutcome {
            url: url.to_string(),
            method: method.to_string(),
            status: meta.status,
            sha256: meta.key_prefix.clone(),
            bytes: meta.bytes,
            fetched_at: meta.fetched_at.clone(),
            from_cache: true,
            content_type: meta.content_type.clone(),
            body,
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
        {
            let mut stats = self.stats.lock().await;
            stats.robots_blocked = stats.robots_blocked.saturating_add(1);
        }
        self.record_access_condition(
            host,
            AccessBlockKind::RobotsDisallowed,
            0,
            None,
            path_and_query.to_string(),
        )
        .await;
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

#[cfg(test)]
mod tests;
