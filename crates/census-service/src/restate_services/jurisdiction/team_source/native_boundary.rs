use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use restate_sdk::prelude::HandlerError;

use super::{admission, ledger};
use crate::restate_services::{blocking, job_error, JobError, Jobs};

mod config;
mod error;
mod files;
mod marker;
#[cfg(test)]
mod tests;

use config::{Config, Selection};
use error::BoundaryError;
use files::Directory;
use marker::Marker;

const ENVIRONMENT: &str = "CENSUS_NATIVE_SOURCE_BOUNDARY";

struct Armed {
    config: Config,
    directory: Directory,
}

use census_crawl::milesplit::boundary;

#[tracing::instrument(skip_all)]
pub(super) async fn wait(
    jobs: &Jobs,
    admission: Arc<admission::WorkAdmission>,
    operation: &str,
    attempt: u8,
    identity: &ledger::Identity,
) -> Result<(), HandlerError> {
    let Some(path) = std::env::var_os(ENVIRONMENT) else {
        return Ok(());
    };
    wait_configured(
        jobs,
        admission,
        PathBuf::from(path),
        operation,
        attempt,
        identity,
    )
    .await
    .map_err(job_error)
}

#[tracing::instrument(skip_all)]
async fn wait_configured(
    jobs: &Jobs,
    admission: Arc<admission::WorkAdmission>,
    path: PathBuf,
    operation: &str,
    attempt: u8,
    identity: &ledger::Identity,
) -> Result<(), JobError> {
    let config_admission = Arc::clone(&admission);
    let armed = blocking(Arc::clone(jobs.region()), move || {
        let _admission = config_admission;
        read_armed(&path)
    })
    .await?;
    let Selection::Hold(timeout) = armed.config.select(operation, attempt) else {
        return Ok(());
    };
    let marker = Marker::new(operation, attempt, identity)?;
    blocking(
        Arc::clone(jobs.region()),
        marker_worker(
            Arc::clone(&admission),
            Arc::clone(jobs.store()),
            armed.directory,
            marker,
            timeout,
        ),
    )
    .await
}

fn read_armed(path: &std::path::Path) -> Result<Armed, BoundaryError> {
    let directory = Directory::open(path)?;
    let filename = path
        .file_name()
        .ok_or(BoundaryError::Path("missing filename"))?;
    if filename == marker::BASENAME || filename == marker::PENDING {
        return Err(BoundaryError::Path(
            "configuration uses a reserved artifact name",
        ));
    }
    let config = Config::read(&directory, &directory.child(filename))?;
    Ok(Armed { config, directory })
}

fn marker_worker(
    admission: Arc<admission::WorkAdmission>,
    store: Arc<census_store::Store>,
    directory: Directory,
    marker: Marker,
    timeout: Duration,
) -> impl FnOnce() -> Result<(), BoundaryError> + Send + 'static {
    move || {
        let _admission = admission;
        store.with_native_effect_checkpoint(|checkpoint| {
            let marker = marker.with_acknowledged_effects(checkpoint);
            marker.publish(&directory)?;
            marker.hold(timeout)
        })?
    }
}

pub struct NativeBoundaryHook;

pub static NATIVE_BOUNDARY_HOOK: NativeBoundaryHook = NativeBoundaryHook {};

impl boundary::Hook for NativeBoundaryHook {
    fn reached<'a>(
        &'a self,
        point: boundary::Point,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), census_crawl::CrawlError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let Some(path) = std::env::var_os(ENVIRONMENT) else {
                return Ok(());
            };
            let path = PathBuf::from(path);
            let config = std::fs::read(&path).map_err(|e| census_crawl::CrawlError::Io {
                path: path.clone(),
                source: e,
            })?;
            let raw: config::RawConfig =
                serde_json::from_slice(&config).map_err(|e| census_crawl::CrawlError::Decode {
                    url: path.display().to_string(),
                    source: e,
                })?;
            let cfg =
                config::Config::parse(raw).map_err(|e| census_crawl::CrawlError::Invariant {
                    detail: e.to_string(),
                })?;
            let selection = cfg.select_point(&point);
            match selection {
                config::Selection::Continue => Ok(()),
                config::Selection::Hold(timeout) => {
                    let directory = files::Directory::open(&path).map_err(|e| {
                        census_crawl::CrawlError::Invariant {
                            detail: e.to_string(),
                        }
                    })?;
                    let identity = ledger::Identity {
                        request_digest: "fault_injection".to_string(),
                        observed_on: "fault_injection".to_string(),
                    };
                    let marker = marker::Marker::new_for_kind(
                        &cfg.operation,
                        cfg.attempt,
                        point.kind(),
                        &identity,
                    )
                    .map_err(|e| census_crawl::CrawlError::Invariant {
                        detail: e.to_string(),
                    })?;
                    marker.publish(&directory).map_err(|e| {
                        census_crawl::CrawlError::Invariant {
                            detail: e.to_string(),
                        }
                    })?;
                    tokio::time::sleep(timeout).await;
                    Err(census_crawl::CrawlError::Invariant {
                        detail: format!("boundary hook timeout at {}", point.kind()),
                    })
                }
            }
        })
    }
}
