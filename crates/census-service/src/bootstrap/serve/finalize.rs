use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use census_store::Store;

use super::super::drain::{attempt_state, count_error, DrainReport, EndpointShutdown};
use super::super::error::BootstrapError;
use super::super::stop::StopReason;
use crate::spawn::Spawner;

pub(super) fn finish_startup(
    watch: Result<(), BootstrapError>,
    finalize: Result<DrainReport, BootstrapError>,
) -> Result<DrainReport, BootstrapError> {
    match (watch, finalize) {
        (Ok(()), Ok(report)) => Ok(report),
        (Err(primary), Ok(_)) => Err(primary),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => Err(BootstrapError::ShutdownCleanup {
            primary: Box::new(primary),
            secondary: Box::new(cleanup),
        }),
    }
}
pub(super) async fn finalize_region(
    store: Arc<Store>,
    region: Arc<Spawner>,
    reason: Arc<AtomicU8>,
    endpoint_shutdown: EndpointShutdown,
    drain_timeout: Duration,
) -> Result<DrainReport, BootstrapError> {
    let drain_result = region.drain(drain_timeout).await.map_err(count_error);
    let attempts_result = attempt_state(&store);
    let flush_result = store.flush().map_err(|source| BootstrapError::StoreFlush {
        source: Box::new(source),
    });
    drop(store);
    match (drain_result, attempts_result, flush_result) {
        (Ok(counted), Ok(attempts), Ok(())) => {
            let mut report = DrainReport::from_counted(counted);
            report.stop_reason = StopReason::from_raw(reason.load(Ordering::SeqCst));
            report.endpoint_shutdown = endpoint_shutdown;
            report.attempts = attempts;
            tracing::info!(?report, "census service stopped");
            Ok(report)
        }
        (Err(primary), Ok(_), Ok(())) => Err(primary),
        (Ok(_), Err(primary), Ok(())) => Err(primary),
        (Ok(_), Ok(_), Err(primary)) => Err(primary),
        (Err(primary), Err(secondary), Ok(())) => Err(BootstrapError::ShutdownCleanup {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        }),
        (Err(primary), Ok(_), Err(secondary)) => Err(BootstrapError::ShutdownCleanup {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        }),
        (Ok(_), Err(primary), Err(secondary)) => Err(BootstrapError::ShutdownCleanup {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        }),
        (Err(first), Err(second), Err(third)) => Err(BootstrapError::ShutdownCleanup {
            primary: Box::new(BootstrapError::ShutdownCleanup {
                primary: Box::new(first),
                secondary: Box::new(second),
            }),
            secondary: Box::new(third),
        }),
    }
}
