pub mod acquisition;
mod artifacts;
mod config;
pub mod control;
pub mod export;
pub mod export_worker;
mod http_audit;
pub mod identity;
pub mod import;
pub mod import_worker;
pub mod profile_worker;
pub mod protocol;
pub mod query_worker;
pub mod review_case;
pub mod reviewer;
pub mod row_protocol;
pub mod row_worker;
pub mod run;
pub mod run_protocol;
pub mod snapshot;
pub mod source;
pub mod source_cache;
pub mod worker;

pub use config::{ExecutionMode, ModelLane, WorkerConfig};

use crate::store::ArtifactStore;
use anyhow::{Context, Result};
use std::{path::Path, sync::Arc};
use tokio::sync::Semaphore;
use tokio_util::task::TaskTracker;

pub struct Runtime {
    pub config: WorkerConfig,
    pub store: ArtifactStore,
    pub http: reqwest::Client,
    cpu: Arc<Semaphore>,
    tasks: TaskTracker,
}

impl Runtime {
    pub fn open(config_path: &Path) -> Result<Arc<Self>> {
        let config = WorkerConfig::load(config_path)?;
        let store = ArtifactStore::open(config.storage_dir())?;
        let http = reqwest::Client::builder()
            .retry(reqwest::retry::never())
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
            cpu,
            tasks: TaskTracker::new(),
        }))
    }

    pub async fn blocking<T, F>(&self, action: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T> + Send + 'static,
    {
        let token = self.tasks.token();
        if self.tasks.is_closed() {
            anyhow::bail!("worker is draining");
        }
        let permit = self.cpu.clone().acquire_owned().await?;
        tokio::task::spawn_blocking(move || {
            let (_token, _permit) = (token, permit);
            action()
        })
        .await
        .context("joining bounded worker action")?
    }

    pub async fn drain(&self) {
        self.cpu.close();
        self.tasks.close();
        self.tasks.wait().await;
    }
}
