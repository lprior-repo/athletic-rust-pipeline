use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{watch, Notify};

pub(super) const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn resident_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    parse_vm_rss_kib(&status).map(|kib| kib.saturating_mul(1024))
}

fn parse_vm_rss_kib(status: &str) -> Option<u64> {
    let rest = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?;
    rest.split_whitespace().next()?.parse::<u64>().ok()
}

pub(super) async fn watch_memory_with(
    reader: impl Fn() -> Option<u64>,
    budget_bytes: u64,
    interval: Duration,
    over_budget: Arc<Notify>,
    mut stopping: watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = stopping.wait_for(|stopping| *stopping) => return,
            _ = ticker.tick() => {}
        }
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

#[tracing::instrument(skip_all, fields(
    budget_gib = budget_bytes / (1024 * 1024 * 1024),
    interval = ?super::guard::SAMPLE_INTERVAL
))]
pub(super) async fn watch_memory(
    budget_bytes: u64,
    over_budget: Arc<Notify>,
    stopping: watch::Receiver<bool>,
) {
    watch_memory_with(
        resident_bytes,
        budget_bytes,
        SAMPLE_INTERVAL,
        over_budget,
        stopping,
    )
    .await
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
        assert!(resident_bytes().is_some_and(|bytes| bytes > 0));
    }

    #[tokio::test(start_paused = true)]
    async fn a_reading_over_budget_trips_the_watcher() {
        let over_budget = Arc::new(Notify::new());
        let reader = || Some(64 * 1024 * 1024 * 1024_u64);
        let (_stop, stopping) = watch::channel(false);
        let watcher = tokio::spawn(watch_memory_with(
            reader,
            48 * 1024 * 1024 * 1024,
            Duration::from_millis(1),
            Arc::clone(&over_budget),
            stopping,
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
        let (_stop, stopping) = watch::channel(false);
        let watcher = tokio::spawn(watch_memory_with(
            reader,
            budget,
            Duration::from_millis(1),
            Arc::clone(&over_budget),
            stopping,
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
