use std::future::pending;

use super::*;

#[tokio::test]
async fn drain_reaps_and_counts_a_task_that_finishes_inside_the_deadline() {
    let spawner = Spawner::new();
    spawner
        .spawn(async {})
        .expect("an empty region admits a task");
    let counted = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 1);
    assert_eq!(counted.completed, 1);
    assert_eq!(counted.timed_out, 0);
    assert_eq!(counted.aborted, 0);
}

#[tokio::test]
async fn drain_aborts_and_counts_what_outlives_the_deadline() {
    let spawner = Spawner::new();
    spawner
        .spawn(pending())
        .expect("an empty region admits a task");
    let counted = spawner
        .drain(Duration::from_millis(1))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 1);
    assert_eq!(counted.completed, 0);
    assert_eq!(counted.timed_out, 1);
    assert_eq!(counted.remaining, 0, "the abort reclaimed the task");
    assert_eq!(counted.aborted, 1, "the reaped cancellation is the abort");
}

#[tokio::test]
async fn drain_counts_a_panicking_task_as_panicked() {
    let spawner = Spawner::new();
    spawner
        .spawn(async {
            panic!("region task is allowed to fail loudly");
        })
        .expect("an empty region admits a task");
    let counted = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.panicked, 1);
    assert_eq!(counted.completed, 0);
}

#[tokio::test]
async fn blocking_returns_the_jobs_value_and_the_jobs_error() {
    let spawner = Spawner::new();
    let value = spawner.blocking(|| Ok::<u8, &'static str>(7)).await;
    assert_eq!(value, Outcome::Ok(7));
    let error = spawner
        .blocking(|| Err::<u8, &'static str>("store refused"))
        .await;
    assert_eq!(error, Outcome::Err("store refused"));
    let counted = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 2);
    assert_eq!(counted.completed, 2);
}

#[tokio::test]
async fn a_running_blocking_job_is_waited_for_and_counted_as_completed() {
    let spawner = Spawner::new();
    let (started, started_rx) = tokio::sync::oneshot::channel::<()>();
    let caller = spawner.blocking(move || {
        let _ = started.send(());
        std::thread::sleep(Duration::from_millis(50));
        Ok::<u8, &'static str>(1)
    });
    let draining = async {
        started_rx
            .await
            .expect("the job announces itself before it finishes");
        spawner
            .drain(Duration::from_millis(1))
            .await
            .expect("a small set fits the report")
    };
    let (outcome, counted) = tokio::join!(caller, draining);
    assert_eq!(counted.timed_out, 1, "the deadline found the job in flight");
    assert_eq!(
        counted.remaining, 1,
        "a blocking job that ran past the deadline was never reaped"
    );
    assert_eq!(
        counted.completed, 0,
        "a job that did not finish inside the deadline is not counted as completed"
    );
    assert_eq!(counted.aborted, 0);
    assert_eq!(outcome, Outcome::Ok(1));
}

#[tokio::test]
async fn the_deadline_reaps_every_abortable_task_and_leaves_the_blocking_job() {
    let spawner = Spawner::new();
    for _ in 0..8 {
        spawner
            .spawn(pending())
            .expect("an empty region admits a task");
    }
    let (started, started_rx) = tokio::sync::oneshot::channel::<()>();
    let caller = spawner.blocking(move || {
        let _ = started.send(());
        std::thread::sleep(Duration::from_millis(300));
        Ok::<u8, &'static str>(1)
    });
    let draining = async {
        started_rx
            .await
            .expect("the job announces itself before it finishes");
        spawner
            .drain(Duration::from_millis(1))
            .await
            .expect("a small set fits the report")
    };
    let (outcome, counted) = tokio::join!(caller, draining);
    assert_eq!(counted.accepted, 9);
    assert_eq!(counted.timed_out, 9);
    assert_eq!(
        counted.aborted, 8,
        "the abort reclaimed every cancellable task"
    );
    assert_eq!(
        counted.remaining, 1,
        "the blocking job the abort cannot reclaim stays counted"
    );
    assert_eq!(counted.completed, 0);
    assert_eq!(outcome, Outcome::Ok(1));
}

#[tokio::test]
async fn drain_returns_promptly_when_task_overruns_deadline() {
    let spawner = Spawner::new();
    spawner
        .spawn(async { tokio::time::sleep(Duration::from_secs(10)).await })
        .expect("an empty region admits a task");
    let start = std::time::Instant::now();
    let counted = spawner
        .drain(Duration::from_millis(10))
        .await
        .expect("a small set fits the report");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(100),
        "drain took {:?} but should return promptly after a 10 ms deadline",
        elapsed
    );
    assert_eq!(counted.timed_out, 1, "the deadline found the job in flight");
    assert_eq!(
        counted.remaining, 0,
        "the abort reclaimed the sleeping task"
    );
    assert_eq!(counted.completed, 0, "not counted as completed");
    assert_eq!(
        counted.aborted, 1,
        "reclaimed by the abort, never completed"
    );
}

#[tokio::test]
async fn blocking_classifies_a_panicking_job_as_panicked() {
    let spawner = Spawner::new();
    let outcome = spawner
        .blocking(|| -> Result<u8, &'static str> { panic!("job is allowed to fail loudly") })
        .await;
    assert_eq!(outcome, Outcome::Panicked);
    let counted = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.panicked, 1, "the region counts the same panic");
}

#[tokio::test]
async fn adopting_counts_the_set_it_was_handed() {
    let mut tasks: JoinSet<()> = JoinSet::new();
    tasks.spawn(async {});
    let spawner = Spawner::adopting(tasks).expect("a small set fits the report");
    let counted = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 1);
    assert_eq!(counted.completed, 1);
}

#[tokio::test]
async fn a_timeout_the_clock_cannot_represent_still_drains() {
    let spawner = Spawner::new();
    spawner
        .spawn(async {})
        .expect("an empty region admits a task");
    let counted = tokio::time::timeout(
        Duration::from_secs(5),
        spawner.drain(Duration::from_secs(u64::MAX)),
    )
    .await
    .expect("the drain returns instead of waiting for an unreachable deadline")
    .expect("a small set fits the report");
    assert_eq!(counted.accepted, 1);
    assert_eq!(counted.completed, 1);
    assert_eq!(counted.timed_out, 0);
}

#[tokio::test]
async fn a_task_started_after_a_drain_belongs_to_the_next_one() {
    let spawner = Spawner::new();
    let first = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("an empty set fits the report");
    assert_eq!(first.accepted, 0);
    spawner
        .spawn(async {})
        .expect("an empty region admits a task");
    let second = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(second.accepted, 1);
    assert_eq!(second.completed, 1);
}

#[tokio::test(start_paused = true)]
async fn a_drain_counts_finished_work_and_the_deadline_separately() {
    let spawner = Spawner::new();
    spawner
        .spawn(async {})
        .expect("an empty region admits a task");
    spawner
        .spawn(pending())
        .expect("an empty region admits a task");
    let counted = spawner
        .drain(Duration::from_millis(1))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 2);
    assert_eq!(
        counted.completed, 1,
        "the task that finished is counted as finished, not as reclaimed"
    );
    assert_eq!(counted.timed_out, 1);
    assert_eq!(counted.remaining, 0, "the abort reclaimed the pending task");
    assert_eq!(counted.aborted, 1);
}

#[tokio::test]
async fn a_full_region_refuses_work_until_a_slot_frees() {
    let spawner = Spawner::with_capacity(1);
    spawner
        .spawn(pending())
        .expect("the first task fits the capacity");
    assert!(matches!(
        spawner.spawn(async {}),
        Err(SpawnError::RegionFull { capacity: 1 })
    ));
    let counted = spawner
        .drain(Duration::from_millis(1))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.timed_out, 1);
    spawner
        .spawn(async {})
        .expect("the drained slot is free again");
    let counted = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 1);
    assert_eq!(counted.completed, 1);
}

#[tokio::test]
async fn the_default_region_admits_its_capacity_and_refuses_the_next_task() {
    let spawner = Spawner::new();
    for _ in 0..DEFAULT_CAPACITY {
        spawner
            .spawn(pending())
            .expect("the configured capacity admits every task up to the bound");
    }
    assert!(matches!(
        spawner.spawn(pending()),
        Err(SpawnError::RegionFull { capacity }) if capacity == DEFAULT_CAPACITY
    ));
    let counted = spawner
        .drain(Duration::from_millis(1))
        .await
        .expect("a small set fits the report");
    let capacity = u64::try_from(DEFAULT_CAPACITY).expect("the capacity fits u64");
    assert_eq!(counted.accepted, capacity);
    assert_eq!(counted.timed_out, capacity);
    assert_eq!(counted.remaining, 0);
    assert_eq!(counted.aborted, capacity);
}

#[tokio::test]
async fn a_full_region_parks_a_blocking_job_until_a_slot_frees() {
    let spawner = Spawner::with_capacity(1);
    spawner
        .spawn(pending())
        .expect("the first task fits the capacity");
    let waited = tokio::time::timeout(
        Duration::from_millis(50),
        spawner.blocking(|| Ok::<u8, &'static str>(7)),
    )
    .await;
    assert!(
        waited.is_err(),
        "a full region parks a blocking job instead of running it early"
    );
    let counted = spawner
        .drain(Duration::from_millis(1))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.timed_out, 1);
    let outcome = spawner.blocking(|| Ok::<u8, &'static str>(7)).await;
    assert_eq!(outcome, Outcome::Ok(7));
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
