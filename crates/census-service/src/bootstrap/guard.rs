//! Process-memory ceiling: the service watches its own resident set and stops when it passes the
//! budget, so an unbounded stage costs a drained, resumable restart instead of the operator's swap.
//!
//! The reading comes from `/proc/self/status` (`VmRSS`): no dependency, no privileged API, and the
//! same figure `ps` reports. Adapters are what actually keep the working set small — the legacy
//! import commits in chunks, a roster is appended as it is parsed — and this guard is the backstop
//! that makes "the process stayed inside its budget" an assertion rather than a hope.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

/// How often the resident set is sampled. Long enough to cost nothing, short enough that a runaway
/// stage is caught while the machine is still responsive.
pub(super) const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// The resident set of this process in bytes, or `None` on a platform that does not report one.
pub(super) fn resident_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    parse_vm_rss_kib(&status).map(|kib| kib.saturating_mul(1024))
}

/// The `VmRSS` line of `/proc/self/status` — `VmRSS:\t  123456 kB` — in kibibytes.
fn parse_vm_rss_kib(status: &str) -> Option<u64> {
    let rest = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?;
    rest.split_whitespace().next()?.parse::<u64>().ok()
}

/// Watch the resident set and trip `over_budget` once it passes `budget_bytes`.
///
/// Returns after tripping, or immediately when the platform reports no resident set; the caller owns
/// the task, so a stop request cancels the wait rather than leaving an orphan watcher.
pub(super) async fn watch_memory_with(
    reader: impl Fn() -> Option<u64>,
    budget_bytes: u64,
    interval: Duration,
    over_budget: Arc<Notify>,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let Some(rss) = reader() else {
            tracing::warn!("no resident-set reading on this platform; the memory guard is off");
            return;
        };
        if rss > budget_bytes {
            tracing::error!(
                rss_gib = rss / (1024 * 1024 * 1024),
                budget_gib = budget_bytes / (1024 * 1024 * 1024),
                "process memory budget exceeded: draining before the machine swaps"
            );
            over_budget.notify_one();
            return;
        }
    }
}

/// Watch the real process resident set against `budget_bytes`.
#[tracing::instrument(skip_all, fields(
    budget_gib = budget_bytes / (1024 * 1024 * 1024),
    interval = ?super::guard::SAMPLE_INTERVAL
))]
pub(super) async fn watch_memory(budget_bytes: u64, over_budget: Arc<Notify>) {
    watch_memory_with(resident_bytes, budget_bytes, SAMPLE_INTERVAL, over_budget).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vm_rss_line_yields_kibibytes() {
        let status = "Name:\tcensus-serve\nVmPeak:\t 1000 kB\nVmRSS:\t  343597 kB\nThreads:\t9\n";
        assert_eq!(parse_vm_rss_kib(status), Some(343_597));
    }

    #[test]
    fn a_status_without_vm_rss_yields_nothing() {
        assert_eq!(parse_vm_rss_kib("Name:\tcensus-serve\n"), None);
    }

    #[test]
    fn this_process_reports_a_resident_set() {
        // The guard is only a backstop if the reading exists where the service runs.
        assert!(resident_bytes().is_some_and(|bytes| bytes > 0));
    }

    #[tokio::test(start_paused = true)]
    async fn a_reading_over_budget_trips_the_watcher() {
        let over_budget = Arc::new(Notify::new());
        let reader = || Some(64 * 1024 * 1024 * 1024_u64);
        let watcher = tokio::spawn(watch_memory_with(
            reader,
            48 * 1024 * 1024 * 1024,
            Duration::from_millis(1),
            Arc::clone(&over_budget),
        ));
        tokio::time::timeout(Duration::from_secs(5), over_budget.notified())
            .await
            .expect("the guard trips when the reading passes the budget");
        watcher.await.expect("the watcher joins after tripping");
    }

    #[tokio::test(start_paused = true)]
    async fn a_reading_within_budget_never_trips() {
        let over_budget = Arc::new(Notify::new());
        let reader = || Some(1024_u64);
        let budget = 48 * 1024 * 1024 * 1024;
        let watcher = tokio::spawn(watch_memory_with(
            reader,
            budget,
            Duration::from_millis(1),
            Arc::clone(&over_budget),
        ));
        assert!(
            tokio::time::timeout(Duration::from_secs(30), over_budget.notified())
                .await
                .is_err(),
            "a process inside its budget is left alone"
        );
        watcher.abort();
    }
}
