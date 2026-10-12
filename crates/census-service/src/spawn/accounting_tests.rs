use super::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn queued_cancellation_cannot_consume_another_tasks_started_credit() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(2)
        .build()?
        .block_on(async {
            let mut pool = JoinSet::new();
            let (occupied, occupancy) = oneshot::channel();
            let (free, freed) = std::sync::mpsc::channel();
            pool.spawn_blocking(move || {
                let _ = occupied.send(());
                freed.recv_timeout(Duration::from_secs(5))
            });
            occupancy.await?;
            let spawner = Spawner::new();
            let (started, began) = oneshot::channel();
            let (release, released) = std::sync::mpsc::channel();
            let running = spawner.blocking(move || {
                let _ = started.send(());
                released.recv_timeout(Duration::from_secs(5)).map(|()| 7)
            });
            tokio::pin!(running);
            tokio::select! {
                outcome = &mut running => return Err(format!("effect finished early: {outcome:?}").into()),
                result = began => result?,
            }
            spawner.spawn(std::future::pending())?;
            let queued = spawner.blocking(|| Ok::<_, ()>(9));
            tokio::pin!(queued);
            tokio::select! {
                biased;
                outcome = &mut queued => return Err(format!("queued effect ran early: {outcome:?}").into()),
                () = std::future::ready(()) => {}
            }
            let interrupted =
                tokio::time::timeout(Duration::from_millis(5), spawner.drain(Duration::ZERO)).await;
            free.send(())?;
            check!(interrupted.is_err(), "started blocking work must stay owned");
            let cancellation = async {
                let outcome = queued.await;
                release.send(())?;
                Ok::<_, Box<dyn std::error::Error>>(outcome)
            };
            let (running, queued, report) =
                tokio::join!(running, cancellation, spawner.drain(Duration::from_secs(5)));
            pool.join_next().await.ok_or("missing pool blocker")???;
            check!(eq; running, Outcome::Ok(7));
            check!(eq; queued?, Outcome::Cancelled);
            let report = report?;
            check!(eq; report.accepted, 3);
            check!(eq; report.completed, 1);
            check!(eq; report.cancelled, 1);
            check!(eq; report.aborted, 1);
            check!(eq; report.panicked, 0);
            check!(eq; report.failed, 0);
            check!(eq; report.still_running, 0);
            check!(eq; report.remaining, 0);
            check!(eq; report.timed_out, 3);
            check!(eq; report.disjoint_total(), report.accepted);
            println!("cancelled drain certificate: {report:?}");
            Ok(())
        })
}

#[test]
fn business_failure_is_a_completed_join_with_a_separate_diagnostic() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            check!(eq; spawner.blocking(|| Err::<(), _>("refused")).await, Outcome::Err("refused"));
            let report = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; report.accepted, 1);
            check!(eq; report.completed, 1);
            check!(eq; report.failed, 1);
            check!(eq; report.disjoint_total(), 1);
            Ok(())
        })
}
