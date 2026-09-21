pub mod acquisition;
mod artifacts;
pub mod browser;
mod browser_config;
mod browser_readiness;
pub mod browser_session;
pub(crate) mod clock;
pub(crate) mod config;
pub mod control;
pub mod drain;
pub mod export;
pub mod export_worker;
mod http_audit;
pub mod identity;
pub mod import;
pub mod import_worker;
mod lifecycle;
pub mod profile_worker;
pub mod protocol;
pub mod query_worker;
pub mod rankings_collection;
pub mod review_case;
pub mod reviewer;
pub mod row_protocol;
pub mod row_worker;
pub mod run;
pub mod run_protocol;
pub mod rankings {
    pub mod catalog;
    pub mod division;
    pub mod page;
    pub mod types;
    pub use super::protocol::{RankingPageObservation, RankingsCapture};
    pub use crate::store::rankings::{
        RankingCandidateEntry, RankingCandidateKind, RankingCollectionStats, RankingEventStats,
        RankingLookup, RankingPageIndex, RankingRecordRef, RankingRosterObservation,
        RankingSourceRow,
    };
    pub use catalog::{EventCatalog, RequestedFamily};
    pub use division::{expected_revision, season_list_id, SeasonKind, SEASON_YEAR};
    pub use page::parse::{parse_page_response, PageParseError};
    pub use types::{
        is_excluded, ExpectedPageContext, IndividualCandidate, NavEvent, PageObservation,
        RankedEvent, RankingsPlan, RankingsScope, RelayMember, RelayRoster, RelayRow, RelayTeam,
        VerifiedRelayMember,
    };
}
pub mod snapshot;
pub mod source;
pub mod source_cache;
pub mod worker;

pub use config::{ExecutionMode, ModelLane, WorkerConfig};

use crate::store::ArtifactStore;
use anyhow::{Context, Result};
use clock::{Clock, SystemClock};
use drain::DrainCounts;
use std::{path::Path, sync::Arc};
use tokio::sync::Semaphore;
use tokio_util::task::TaskTracker;

pub struct Runtime {
    pub config: WorkerConfig,
    pub store: ArtifactStore,
    pub http: reqwest::Client,
    browser: tokio::sync::RwLock<Option<Arc<browser::BrowserManager>>>,
    cpu: Arc<Semaphore>,
    tasks: TaskTracker,
    drain_counts: Arc<DrainCounts>,
    clock: Arc<dyn Clock>,
}

impl Runtime {
    pub fn open(config_path: &Path) -> Result<Arc<Self>> {
        let config = WorkerConfig::load(config_path)?;
        let store = ArtifactStore::open(config.storage_dir())?;
        let http = reqwest::Client::builder()
            .retry(reqwest::retry::never())
            .no_gzip()
            .no_brotli()
            .no_zstd()
            .no_deflate()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(config.request_timeout())
            .user_agent("athletic-rust-pipeline/0.1 authorized-research")
            .build()
            .context("building bounded HTTP client")?;
        let cpu = Arc::new(Semaphore::new(config.cpu_workers()));
        Ok(Arc::new(Self {
            config,
            store,
            http,
            browser: tokio::sync::RwLock::new(None),
            cpu,
            tasks: TaskTracker::new(),
            drain_counts: Arc::new(DrainCounts::default()),
            clock: Arc::new(SystemClock),
        }))
    }

    /// The one clock every deadline, cooldown and timeout in this runtime reads.
    pub(crate) fn clock(&self) -> Arc<dyn Clock> {
        self.clock.clone()
    }

    pub(crate) async fn browser(&self) -> Option<Arc<browser::BrowserManager>> {
        self.browser.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::drain::DrainReport;
    use std::time::Duration;
    use tempfile::tempdir;

    fn config(path: &Path) -> Arc<Runtime> {
        // The store root must be created by the store: `prepare_root` makes it private (0o700),
        // while a path handed in already existing is only validated, and a fixture directory is
        // world-readable by construction.
        let text = format!(
            "mode = \"fixture\"\nstorage_dir = \"{}\"\nsource_origin = \"http://127.0.0.1/\"\nsource_interval_ms = 0\nrequest_timeout_seconds = 1\ncpu_workers = 1\nrow_concurrency = 1\nq5_url = \"http://127.0.0.1/\"\nq5_model = \"q5\"\nq4_url = \"http://127.0.0.1/\"\nq4_model = \"q4\"\n",
            path.join("store").display()
        );
        let file = path.join("worker.toml");
        std::fs::write(&file, text).expect("synthetic worker config");
        Runtime::open(&file).expect("runtime opens on a synthetic config")
    }

    #[tokio::test]
    async fn a_finished_blocking_unit_is_certified_as_completed() {
        let directory = tempdir().expect("temporary storage");
        let runtime = config(directory.path());
        assert_eq!(runtime.blocking(|| Ok(7)).await.expect("action runs"), 7);

        let report = runtime.drain_within(Duration::from_secs(5)).await;
        assert_eq!(
            report,
            DrainReport {
                accepted: 1,
                completed: 1,
                ..DrainReport::default()
            }
        );
    }

    #[tokio::test]
    async fn a_panicking_blocking_unit_is_certified_as_panicked() {
        let directory = tempdir().expect("temporary storage");
        let runtime = config(directory.path());
        let result = runtime
            .blocking(|| -> Result<()> { panic!("bounded action panics") })
            .await;
        assert!(result.is_err(), "a panicking action does not return a value");

        let report = runtime.drain_within(Duration::from_secs(5)).await;
        assert_eq!(report.accepted, 1);
        assert_eq!(report.panicked, 1);
        assert_eq!(report.completed, 0);
        assert_eq!(report.remaining, 0);
    }

    #[tokio::test]
    async fn a_unit_still_running_at_the_deadline_stays_remaining() {
        let directory = tempdir().expect("temporary storage");
        let runtime = config(directory.path());
        let (entered, started) = tokio::sync::oneshot::channel();
        let worker = Arc::clone(&runtime);
        let running = tokio::spawn(async move {
            worker
                .blocking(move || {
                    let _ = entered.send(());
                    std::thread::sleep(Duration::from_millis(300));
                    Ok(())
                })
                .await
        });
        started.await.expect("the blocking unit started");

        let report = runtime.drain_within(Duration::from_millis(20)).await;
        assert_eq!(report.accepted, 1);
        assert_eq!(report.timed_out, 1);
        assert_eq!(report.remaining, 1);
        assert_eq!(report.completed, 0);

        assert!(running.await.expect("join").is_ok(), "the unit still runs");
        assert!(
            runtime.blocking(|| Ok(())).await.is_err(),
            "admission is closed"
        );
    }
}
