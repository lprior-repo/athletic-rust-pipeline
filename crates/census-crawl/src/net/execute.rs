use super::cache::{read_cache, CacheMeta};
use super::client::HostState;
use super::request::RequestBody;
use super::{FetchError, FetchOptions, FetchOutcome, Fetcher, MIN_AUTHORIZED_DELAY};
use census_store::clock::{Clock, SystemClock};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

mod attempt;
mod attempt_helper;
mod body_reader;
mod browser;
mod cache_writer;
mod representation;

use attempt::FetchPlan;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Family,
    Host,
}

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
    fn pace_scope(&self, host: &str) -> PaceScope {
        let host = host.trim().to_ascii_lowercase();
        match self.family_of(&host) {
            Some(family) if self.family_parallelism <= 1 => PaceScope::Family(family),
            _ => PaceScope::Host(host),
        }
    }

    fn configured_delay(&self, scope: &PaceScope) -> Duration {
        match scope {
            PaceScope::Family(family) => self
                .family_delay(family)
                .map_or(self.default_delay, |value| value),
            PaceScope::Host(host) => {
                let family_delay = self
                    .family_of(host)
                    .and_then(|family| self.family_delay(&family));
                let host_delay = self.host_delays.get(host).copied();
                family_delay
                    .max(host_delay)
                    .map_or(self.default_delay, |value| value)
            }
        }
    }

    pub(super) async fn host_gate(&self, host: &str) -> Arc<Mutex<()>> {
        let scope = self.pace_scope(host);
        let mut effective = self.configured_delay(&scope);
        if self.is_authorized_host(host) && effective < MIN_AUTHORIZED_DELAY {
            effective = MIN_AUTHORIZED_DELAY;
        }
        if let Some(declared) = crate::registry::declared_delay_for_host(host) {
            effective = effective.max(declared);
        }
        let kind = scope.kind();
        let key = scope.into_key();
        let mut buckets = match kind {
            ScopeKind::Family => self.pacing.families.lock().await,
            ScopeKind::Host => self.pacing.hosts.lock().await,
        };
        let state = buckets.entry(key).or_insert_with(|| HostState {
            gate: Arc::new(Mutex::new(())),
            next_allowed: None,
            delay: effective,
        });
        state.delay = state.delay.max(effective);
        state.gate.clone()
    }

    pub(super) async fn wait_turn(&self, host: &str) {
        let scope = self.pace_scope(host);
        let kind = scope.kind();
        let key = scope.into_key();
        let wait = {
            let mut buckets = match kind {
                ScopeKind::Family => self.pacing.families.lock().await,
                ScopeKind::Host => self.pacing.hosts.lock().await,
            };
            match buckets.get_mut(&key) {
                Some(s) => {
                    let now = SystemClock.now();
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
    pub(super) async fn fetch(
        &self,
        method: &str,
        url: &str,
        body: Option<(String, RequestBody)>,
        options: &FetchOptions,
        timeout_secs: u64,
    ) -> Result<FetchOutcome, FetchError> {
        self.destination.validate_url(url)?;
        let representation = representation::canonical_request(&options.headers)?;
        let extra = representation::cache_extra(body.as_ref(), &representation);
        let key = Self::key_for(method, url, &extra);
        let (body_path, meta_path) = self.cache_paths(&key);
        let cached = read_cache(&body_path, &meta_path, method, url, &representation)?;
        if let Some((meta, body)) = cached.as_ref() {
            if let Some(outcome) = self
                .cached_outcome(method, url, meta, options, body.clone())
                .await?
            {
                return Ok(outcome);
            }
        }
        if self.offline {
            return Err(FetchError::Offline {
                url: url.to_string(),
            });
        }

        let (host, origin) = request_target(url)?;
        self.origin_locks.ensure(&origin)?;
        if self.host_blocked(&host, &super::now_iso8601()).await {
            return Err(FetchError::Cooldown { host: host.clone() });
        }
        let gate = self.host_gate(&host).await;
        let plan = FetchPlan {
            method,
            url,
            payload: body.as_ref().map(|(_, payload)| payload),
            host: &host,
            family: self.family_of(&host),
            body_path: &body_path,
            meta_path: &meta_path,
            cached: cached.as_ref().map(|(meta, _)| meta),
            options,
            representation: &representation,
            timeout_secs,
        };
        let _family_permit = self.family_permit(&host).await?;
        let started = Instant::now();
        let outcome = match crate::registry::transport_for_host(&host) {
            Some(crate::registry::TransportKind::Browser) => self.fetch_browser(gate, &plan).await,
            _ => self.fetch_once(&plan).await,
        };
        self.record_transport(started, &outcome).await;
        outcome
    }

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
            stats.requests = stats.requests.saturating_add(1);
            let entry = stats.per_host.entry(crate::net::host_of(url)).or_default();
            entry.requests = entry.requests.saturating_add(1);
            entry.cache_hits = entry.cache_hits.saturating_add(1);
        }
        Ok(Some(FetchOutcome {
            url: url.to_string(),
            response_url: meta.response_url.clone(),
            method: method.to_string(),
            status: meta.status,
            content_digest: meta.content_digest.clone(),
            bytes: meta.bytes,
            fetched_at: meta.fetched_at.clone(),
            from_cache: true,
            content_type: meta.content_type.clone(),
            body,
        }))
    }
}

fn request_target(url: &str) -> Result<(String, String), FetchError> {
    let parsed = url::Url::parse(url).map_err(|source| FetchError::InvalidUrl {
        url: url.to_string(),
        source,
    })?;
    let host = parsed
        .host_str()
        .map_or(Default::default(), core::convert::identity)
        .to_string();
    let origin = format!(
        "{}://{}",
        parsed.scheme(),
        match parsed.port().map(|p| format!("{host}:{p}")) {
            Some(value) => value,
            None => host.clone(),
        }
    );
    Ok((host, origin))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "execute/conditional_capture_tests.rs"]
mod conditional_capture;

#[cfg(test)]
mod response_url_tests;

#[cfg(test)]
mod acquisition_regressions;
