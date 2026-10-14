use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{watch, Notify};

mod cgroup;
mod kernel_file;
mod reading;
pub(super) use reading::MemoryReader;

pub(super) const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn resident_bytes() -> Option<u64> {
    kernel_file::with_bytes::<16385, _>(std::path::Path::new("/proc/self/status"), |bytes| {
        let status = std::str::from_utf8(bytes).ok()?;
        parse_vm_rss_kib(status)?.checked_mul(1024)
    })
    .ok()?
}

fn parse_vm_rss_kib(status: &str) -> Option<u64> {
    let rest = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))?;
    let mut fields = rest.split_whitespace();
    let kib = fields.next()?.parse::<u64>().ok()?;
    if fields.next()? != "kB" || fields.next().is_some() {
        return None;
    }
    Some(kib)
}

pub(super) async fn watch_memory_with(
    mut reader: impl FnMut() -> Option<u64>,
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
            tracing::error!("memory reading unavailable; draining instead of disabling accounting");
            over_budget.notify_one();
            return;
        };
        if rss > budget_bytes {
            tracing::error!(
                memory_bytes = rss,
                budget_bytes,
                "memory budget exceeded: draining"
            );
            over_budget.notify_one();
            return;
        }
    }
}

#[tracing::instrument(skip_all, fields(
    budget_bytes = reader.budget_bytes(),
    interval = ?super::guard::SAMPLE_INTERVAL
))]
pub(super) async fn watch_memory(
    mut reader: MemoryReader,
    over_budget: Arc<Notify>,
    stopping: watch::Receiver<bool>,
) {
    let budget_bytes = reader.budget_bytes();
    watch_memory_with(
        || reader.read(),
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

    #[test]
    fn a_reading_over_budget_trips_the_watcher() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?
            .block_on(async {
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
                tokio::time::timeout(Duration::from_secs(5), over_budget.notified()).await?;
                watcher.await?;
                Ok(())
            })
    }

    #[test]
    fn a_reading_within_budget_never_trips() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?
            .block_on(async {
                let over_budget = Arc::new(Notify::new());
                let reader = || Some(1024_u64);
                let budget = 48 * 1024 * 1024 * 1024;
                let (stop, stopping) = watch::channel(false);
                let watcher = tokio::spawn(watch_memory_with(
                    reader,
                    budget,
                    Duration::from_millis(1),
                    Arc::clone(&over_budget),
                    stopping,
                ));
                let outcome =
                    tokio::time::timeout(Duration::from_secs(30), over_budget.notified()).await;
                stop.send(true)?;
                watcher.await?;
                check!(
                    outcome.is_err(),
                    "a process inside its budget is left alone"
                );
                Ok(())
            })
    }

    #[test]
    fn an_unavailable_memory_reading_requests_a_controlled_stop(
    ) -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .start_paused(true)
            .build()?
            .block_on(async {
                let over_budget = Arc::new(Notify::new());
                let (_stop, stopping) = watch::channel(false);
                let watcher = tokio::spawn(watch_memory_with(
                    || None,
                    1024,
                    Duration::from_millis(1),
                    Arc::clone(&over_budget),
                    stopping,
                ));
                let stopped =
                    tokio::time::timeout(Duration::from_secs(1), over_budget.notified()).await;
                watcher.await?;
                stopped?;
                Ok(())
            })
    }
}
