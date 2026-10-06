use std::future::pending;
use std::time::Duration;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn panic_region_task_fault() {
    panic!("region task is allowed to fail loudly");
}

fn panic_blocking_job_fault() -> ! {
    panic!("job is allowed to fail loudly");
}

#[test]
fn drain_reaps_and_counts_a_task_that_finishes_inside_the_deadline() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            spawner.spawn(async {})?;
            let counted = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; counted.accepted, 1);
            check!(eq; counted.completed, 1);
            check!(eq; counted.timed_out, 0);
            check!(eq; counted.aborted, 0);
            Ok(())
        })
}

#[test]
fn drain_aborts_and_counts_what_outlives_the_deadline() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            spawner.spawn(pending())?;
            let counted = spawner.drain(Duration::from_millis(1)).await?;
            check!(eq; counted.accepted, 1);
            check!(eq; counted.completed, 0);
            check!(eq; counted.timed_out, 1);
            check!(eq; counted.remaining, 0, "the abort reclaimed the task");
            check!(eq; counted.aborted, 1, "the reaped cancellation is the abort");
            Ok(())
        })
}

#[test]
fn drain_counts_a_panicking_task_as_panicked() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            spawner.spawn(async {
                panic_region_task_fault();
            })?;
            let counted = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; counted.panicked, 1);
            check!(eq; counted.completed, 0);
            Ok(())
        })
}

#[test]
fn blocking_returns_the_jobs_value_and_the_jobs_error() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            let value = spawner.blocking(|| Ok::<u8, &'static str>(7)).await;
            check!(eq; value, Outcome::Ok(7));
            let error = spawner
                .blocking(|| Err::<u8, &'static str>("store refused"))
                .await;
            check!(eq; error, Outcome::Err("store refused"));
            let counted = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; counted.accepted, 2);
            check!(eq; counted.completed, 2);
            Ok(())
        })
}

#[test]
fn a_running_blocking_job_is_waited_for_and_counted_as_completed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            let (started, started_rx) = tokio::sync::oneshot::channel::<()>();
            let caller = spawner.blocking(move || {
                let _ = started.send(());
                std::thread::sleep(Duration::from_millis(50));
                Ok::<u8, &'static str>(1)
            });
            let draining = async {
                started_rx.await?;
                Ok::<_, Box<dyn std::error::Error>>(spawner.drain(Duration::from_millis(1)).await?)
            };
            let (outcome, counted) = tokio::join!(caller, draining);
            let counted = counted?;
            check!(eq; counted.accepted, 1);
            check!(eq; counted.timed_out, 1, "the deadline found the job in flight");
            check!(eq;
                counted.remaining, 0,
                "the started blocking job was reaped before drain returned"
            );
            check!(eq;
                counted.completed, 1,
                "a blocking job that started before the deadline runs to completion"
            );
            check!(eq; counted.aborted, 0);
            check!(eq; outcome, Outcome::Ok(1));
            Ok(())
        })
}

#[test]
fn drain_returns_promptly_when_task_overruns_deadline() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            spawner.spawn(async { tokio::time::sleep(Duration::from_secs(10)).await })?;
            let start = std::time::Instant::now();
            let counted = spawner.drain(Duration::from_millis(10)).await?;
            let elapsed = start.elapsed();
            check!(
                elapsed < Duration::from_millis(100),
                "drain took {:?} but should return promptly after a 10 ms deadline",
                elapsed
            );
            check!(eq; counted.timed_out, 1, "the deadline found the job in flight");
            check!(eq;
                counted.remaining, 0,
                "the abort reclaimed the sleeping task"
            );
            check!(eq; counted.completed, 0, "not counted as completed");
            check!(eq;
                counted.aborted, 1,
                "reclaimed by the abort, never completed"
            );
            Ok(())
        })
}

#[test]
fn blocking_classifies_a_panicking_job_as_panicked() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            let outcome = spawner
                .blocking(|| -> Result<u8, &'static str> { panic_blocking_job_fault() })
                .await;
            check!(eq; outcome, Outcome::Panicked);
            let counted = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; counted.panicked, 1, "the region counts the same panic");
            Ok(())
        })
}

#[test]
fn adopting_counts_the_set_it_was_handed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut tasks: JoinSet<()> = JoinSet::new();
            tasks.spawn(async {});
            let spawner = Spawner::adopting(tasks)?;
            let counted = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; counted.accepted, 1);
            check!(eq; counted.completed, 1);
            Ok(())
        })
}

#[test]
fn a_timeout_the_clock_cannot_represent_still_drains() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            spawner.spawn(async {})?;
            let counted = tokio::time::timeout(
                Duration::from_secs(5),
                spawner.drain(Duration::from_secs(u64::MAX)),
            )
            .await??;
            check!(eq; counted.accepted, 1);
            check!(eq; counted.completed, 1);
            check!(eq; counted.timed_out, 0);
            Ok(())
        })
}

#[test]
fn a_drain_closes_admission_for_good() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            let first = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; first.accepted, 0);
            check!(matches!(
                spawner.spawn(async {}),
                Err(SpawnError::RegionClosed)
            ));
            check!(eq;
                spawner.blocking(|| Ok::<u8, &'static str>(7)).await,
                Outcome::Cancelled,
                "a closed region runs no new blocking effect"
            );
            let second = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; second.accepted, 0);
            check!(eq; second.completed, 0);
            check!(eq; second.timed_out, 0);
            Ok(())
        })
}

#[test]
fn a_drain_counts_finished_work_and_the_deadline_separately() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            spawner.spawn(async {})?;
            spawner.spawn(pending())?;
            let counted = spawner.drain(Duration::from_millis(1)).await?;
            check!(eq; counted.accepted, 2);
            check!(eq;
                counted.completed, 1,
                "the task that finished is counted as finished, not as reclaimed"
            );
            check!(eq; counted.timed_out, 1);
            check!(eq; counted.remaining, 0, "the abort reclaimed the pending task");
            check!(eq; counted.aborted, 1);
            Ok(())
        })
}

#[test]
fn a_full_region_refuses_work_until_a_slot_frees() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::with_capacity(1);
            let (release, released) = oneshot::channel::<()>();
            spawner.spawn(async move {
                let _ = released.await;
            })?;
            check!(matches!(
                spawner.spawn(async {}),
                Err(SpawnError::RegionFull { capacity: 1 })
            ));
            let mut parked = Box::pin(spawner.blocking(|| Ok::<u8, &'static str>(7)));
            tokio::select! {
                biased;
                outcome = &mut parked => return Err(format!("a full region ran the job early: {outcome:?}").into()),
                () = std::future::ready(()) => {}
            }
            let _ = release.send(());
            let outcome = tokio::time::timeout(Duration::from_secs(5), &mut parked)
                .await
                .map_err(|_| "the freed slot did not admit the parked job")?;
            check!(eq; outcome, Outcome::Ok(7));
            let counted = spawner.drain(Duration::from_secs(5)).await?;
            check!(eq; counted.accepted, 2);
            check!(eq; counted.completed, 2);
            Ok(())
        })
}

#[test]
fn the_default_region_admits_its_capacity_and_refuses_the_next_task() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let spawner = Spawner::new();
            for _ in 0..DEFAULT_CAPACITY {
                spawner.spawn(pending())?;
            }
            check!(matches!(
                spawner.spawn(pending()),
                Err(SpawnError::RegionFull { capacity }) if capacity == DEFAULT_CAPACITY
            ));
            let counted = spawner.drain(Duration::from_millis(1)).await?;
            let capacity = u64::try_from(DEFAULT_CAPACITY)?;
            check!(eq; counted.accepted, capacity);
            check!(eq; counted.timed_out, capacity);
            check!(eq; counted.remaining, 0);
            check!(eq; counted.aborted, capacity);
            Ok(())
        })
}

#[test]
fn counts_saturate_instead_of_wrapping() {
    let mut ledger = super::ledger::Ledger::holding(u64::MAX);
    ledger.accept();
    assert_eq!(
        ledger.report().accepted,
        u64::MAX,
        "a count at its ceiling stays there"
    );
}
