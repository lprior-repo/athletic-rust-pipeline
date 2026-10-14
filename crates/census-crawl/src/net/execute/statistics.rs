use crate::net::{FetchError, FetchOutcome, Fetcher};
use std::time::Instant;

impl Fetcher {
    pub(super) async fn count_attempt(&self, host: &str) {
        let mut stats = self.stats.lock().await;
        stats.requests = stats.requests.saturating_add(1);
        if let Some(entry) = stats.per_host.get_mut(host) {
            entry.requests = entry.requests.saturating_add(1);
        } else {
            stats.per_host.entry(host.to_string()).or_default().requests = 1;
        }
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
}
