use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn reaped_jobs_and_rejected_work_balance_without_counting_timeout_twice() -> TestResult {
    let mut report = DrainReport::default();
    report.accept(6)?;
    report.cancel(2)?;
    report.record(&Outcome::<(), ()>::Ok(()))?;
    report.record(&Outcome::<(), ()>::Err(()))?;
    report.record(&Outcome::<(), ()>::Timeout)?;
    let cancelled = tokio::spawn(std::future::pending::<()>());
    cancelled.abort();
    report.record_killed(cancelled.await)?;
    let panicked = tokio::spawn(controlled_drain_fault());
    report.record_killed(panicked.await)?;
    check!(eq; report, DrainReport {
        accepted: 6,
        completed: 2,
        cancelled: 2,
        timed_out: 1,
        aborted: 1,
        panicked: 1,
        remaining: 0,
    });
    Ok(())
}

#[test]
fn unresolved_work_stays_owed_and_timeout_is_only_a_diagnostic() -> TestResult {
    let mut report = DrainReport::default();
    report.accept(3)?;
    report.record(&Outcome::<(), ()>::Ok(()))?;
    report.abandon(2)?;
    check!(eq; report, DrainReport {
        accepted: 3,
        completed: 1,
        remaining: 2,
        timed_out: 2,
        ..DrainReport::default()
    });
    Ok(())
}

#[test]
fn overflow_never_changes_any_part_of_the_certificate() -> TestResult {
    type Mutation = fn(&mut DrainReport) -> Result<(), DrainAccountingError>;
    let cases: [(DrainReport, Mutation); 6] = [
        (
            DrainReport {
                accepted: u64::MAX,
                ..DrainReport::default()
            },
            |r| r.accept(1),
        ),
        (
            DrainReport {
                cancelled: u64::MAX,
                ..DrainReport::default()
            },
            |r| r.cancel(1),
        ),
        (
            DrainReport {
                completed: u64::MAX,
                ..DrainReport::default()
            },
            |r| r.record(&Outcome::<(), ()>::Ok(())),
        ),
        (
            DrainReport {
                cancelled: u64::MAX,
                ..DrainReport::default()
            },
            |r| r.record(&Outcome::<(), ()>::Cancelled),
        ),
        (
            DrainReport {
                timed_out: u64::MAX,
                ..DrainReport::default()
            },
            |r| r.record(&Outcome::<(), ()>::Timeout),
        ),
        (
            DrainReport {
                panicked: u64::MAX,
                ..DrainReport::default()
            },
            |r| r.record(&Outcome::<(), ()>::Panicked),
        ),
    ];
    for (mut report, mutate) in cases {
        let before = report.clone();
        check!(eq; mutate(&mut report), Err(DrainAccountingError::Overflow));
        check!(eq; report, before);
    }
    Ok(())
}

#[test]
fn abandonment_is_atomic_when_either_field_cannot_advance() -> TestResult {
    for mut report in [
        DrainReport {
            remaining: u64::MAX,
            timed_out: 7,
            ..DrainReport::default()
        },
        DrainReport {
            remaining: 7,
            timed_out: u64::MAX,
            ..DrainReport::default()
        },
    ] {
        let before = report.clone();
        check!(eq; report.abandon(1), Err(DrainAccountingError::Overflow));
        check!(eq; report, before);
    }
    Ok(())
}

async fn controlled_drain_fault() {
    panic!("controlled drain fault");
}
