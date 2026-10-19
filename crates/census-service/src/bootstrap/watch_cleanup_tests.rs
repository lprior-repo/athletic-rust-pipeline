use super::ServeOptions;
use crate::bootstrap::{BootstrapError, ServeCaptureProtocol};
use crate::spawn::Spawner;
use std::sync::Arc;
use std::time::Duration;
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[test]
fn full_region_watch_failure_still_releases_store_and_restores_credits() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::TempDir::new()?;
            let data_dir = dir.path().join("store");
            census_store::Store::create_current_qualification_v3(&data_dir)?;
            let region = Arc::new(Spawner::with_capacity(1));
            let probe = Arc::clone(&region);
            let memory = crate::bootstrap::guard::MemoryReader::prepare(&ServeOptions::default())?;
            let options = ServeOptions {
                listen: "127.0.0.1:0".parse()?,
                data_dir: data_dir.clone(),
                capture_protocol: Some(ServeCaptureProtocol::FreshQualificationV3),
                max_concurrent: 1,
                drain_timeout: Duration::from_secs(5),
                ..ServeOptions::default()
            };
            let outcome = tokio::time::timeout(
                Duration::from_secs(30),
                super::supervise_region(options, std::future::pending::<()>(), memory, region),
            )
            .await?;
            match outcome {
                Err(BootstrapError::TaskSupervision { source }) => match *source {
                    crate::spawn::SpawnError::RegionFull { .. }
                    | crate::spawn::SpawnError::Admission(_) => {}
                    ref other => {
                        return Err(
                            format!("watcher primary must stay RegionFull, got {other:?}").into(),
                        )
                    }
                },
                Err(other) => {
                    return Err(format!(
                        "watcher failure must preserve TaskSupervision primary, got {other:?}"
                    )
                    .into())
                }
                Ok(_) => return Err("watcher failure must not report success".into()),
            }
            let reopened = census_store::Store::open_current_qualification_v3(&data_dir)?;
            drop(reopened);
            if probe.coordinator().remaining().slots.tasks != 1 {
                return Err(format!(
                    "endpoint credits must be restored, tasks={}",
                    probe.coordinator().remaining().slots.tasks
                )
                .into());
            }
            let report = probe.drain(Duration::ZERO).await?;
            if report.remaining != 0 || report.accepted != 0 {
                return Err(format!(
                    "task report must be restored, accepted={} remaining={}",
                    report.accepted, report.remaining
                )
                .into());
            }
            Ok(())
        })
}
