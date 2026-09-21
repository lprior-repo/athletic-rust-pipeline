//! Unit tests for the region spawner: the counting discipline, the outcome classification, and
//! the fact that a drain owns everything the region started.

use std::future::pending;

use super::*;

#[tokio::test]
async fn drain_reaps_and_counts_a_task_that_finishes_inside_the_deadline() {
    let spawner = Spawner::new();
    spawner.spawn(async {});
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
    spawner.spawn(pending());
    let counted = spawner
        .drain(Duration::from_millis(1))
        .await
        .expect("a small set fits the report");
    assert_eq!(counted.accepted, 1);
    assert_eq!(counted.completed, 0);
    assert_eq!(counted.timed_out, 1);
    assert_eq!(counted.remaining, 1);
    assert_eq!(counted.aborted, 1);
}

#[tokio::test]
async fn drain_counts_a_panicking_task_as_panicked() {
    let spawner = Spawner::new();
    spawner.spawn(async {
        panic!("region task is allowed to fail loudly");
    });
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
    // The job announces itself from the pool thread, so the drain below cannot start before the job
    // is genuinely in flight: a job that has started cannot be aborted, and the drain has to wait
    // for it instead of returning with a writer still running.
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
        counted.completed, 1,
        "a job that finished is counted as finished, not as aborted"
    );
    assert_eq!(counted.aborted, 0);
    assert_eq!(outcome, Outcome::Ok(1));
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
    spawner.spawn(async {});
    // A deadline no instant can hold: the drain has no deadline to reach, and that is a wait, not a
    // panicked `now + timeout` and not an immediate abort. The outer bound is the test's own, so a
    // platform whose clock *can* hold this waits the region out instead of hanging the suite.
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
    spawner.spawn(async {});
    let second = spawner
        .drain(Duration::from_secs(5))
        .await
        .expect("a small set fits the report");
    assert_eq!(second.accepted, 1);
    assert_eq!(second.completed, 1);
}
