use super::*;
use crate::spawn::TaskReport;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn endpoint_cancellation_signals_completion_and_drains_the_original_single_owned_task() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let store = Arc::new(Store::open(root.path())?);
            let region = Arc::new(Spawner::new());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let bound = listener.local_addr()?;
            let options = ServeOptions::default();
            let (cancel, done) = super::spawn_endpoint(
                &region,
                &store,
                CaptureExecution::LegacyFresh,
                &options,
                listener,
                None,
            )?;
            cancel
                .send(())
                .map_err(|_| "endpoint lost cancellation receiver")?;
            tokio::time::timeout(Duration::from_secs(5), done).await??;
            let report = region.drain(Duration::from_secs(5)).await?;
            assert_eq!(
                report,
                TaskReport {
                    accepted: 1,
                    completed: 1,
                    ..TaskReport::default()
                }
            );
            assert_eq!(report.disjoint_total(), 1);
            let rebound = tokio::net::TcpListener::bind(bound).await?;
            assert_eq!(rebound.local_addr()?, bound);
            Ok(())
        })
}

#[test]
fn endpoint_concurrency_refusal_drops_listener_without_admitting_a_task() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let store = Arc::new(Store::open(root.path())?);
            let region = Arc::new(Spawner::new());
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let bound = listener.local_addr()?;
            let options = ServeOptions {
                max_concurrent: 0,
                ..ServeOptions::default()
            };
            match super::spawn_endpoint(
                &region,
                &store,
                CaptureExecution::LegacyFresh,
                &options,
                listener,
                None,
            ) {
                Err(BootstrapError::ConcurrencyTooLarge { value, ceiling }) => {
                    assert_eq!(value, 0);
                    assert_eq!(ceiling, crate::restate_services::MAX_ENDPOINT_CONCURRENCY);
                }
                other => return Err(format!("invalid endpoint concurrency={other:?}").into()),
            }
            assert_eq!(
                region.drain(Duration::from_secs(5)).await?,
                TaskReport::default()
            );
            let rebound = tokio::net::TcpListener::bind(bound).await?;
            assert_eq!(rebound.local_addr()?, bound);
            Ok(())
        })
}
