use super::client::HostState;
use super::{FetchError, Fetcher, MIN_AUTHORIZED_DELAY};
use census_store::clock::{Clock, SystemClock};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

mod admission;
mod attempt;
mod attempt_helper;
mod body_reader;
mod browser;
mod cache_writer;
mod dispatch;
mod redirect;
pub(super) mod representation;
mod statistics;

mod fetch;

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
    #[tracing::instrument(skip(self))]
    pub(super) async fn raise_robots_delay(
        &self,
        host: &str,
        delay: Duration,
    ) -> Result<(), FetchError> {
        let _gate = self.host_gate(host).await;
        let scope = self.pace_scope(host);
        let kind = scope.kind();
        let key = scope.into_key();
        let mut buckets = match kind {
            ScopeKind::Family => self.pacing.families.lock().await,
            ScopeKind::Host => self.pacing.hosts.lock().await,
        };
        let state = buckets.get_mut(&key).ok_or_else(|| FetchError::Invariant {
            detail: "robots pacing scope disappeared".to_owned(),
        })?;
        state.delay = state.delay.max(delay);
        let earliest =
            SystemClock
                .now()
                .checked_add(state.delay)
                .ok_or_else(|| FetchError::Policy {
                    detail: "robots pacing deadline overflowed".to_owned(),
                })?;
        state.next_allowed = Some(state.next_allowed.map_or(earliest, |at| at.max(earliest)));
        Ok(())
    }

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
#[path = "execute/attempt_helper_tests.rs"]
mod attempt_helper_tests;

#[cfg(test)]
#[path = "execute/conditional_capture_tests.rs"]
mod conditional_capture;

#[cfg(test)]
mod response_url_tests;

#[cfg(test)]
mod acquisition_regressions;

#[cfg(test)]
mod source_header_tests;
