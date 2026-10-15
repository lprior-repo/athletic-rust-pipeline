use super::*;
use crate::spawn::Spawner;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn coordinator(bytes: u64, slots: u64, waiters: u64) -> TestResult<Coordinator> {
    Ok(Coordinator::with(
        bytes,
        1_000_000,
        slots,
        waiters,
        WORK_CLASS_NORMAL,
    )?)
}

fn grant(result: TryResult<Reservation>) -> TestResult<Reservation> {
    match result {
        TryResult::Granted(reservation) => Ok(reservation),
        TryResult::Deferred(reason) | TryResult::Refused(reason) => Err(reason.into()),
    }
}

fn profile(bytes: u64, slots: u64) -> WorkClass {
    WorkClass::work(
        bytes,
        1,
        SlotVector {
            tasks: slots,
            stages: 0,
            browser: 0,
            populations: 0,
        },
    )
}

#[test]
fn empty_resource_envelope_is_refused() -> TestResult {
    let coordinator = coordinator(1024, 1, 4)?;
    check!(matches!(
        coordinator.try_reserve_profile(WorkClass::work(0, 0, SlotVector::default())),
        TryResult::Refused(DepartureReason::EmptyEnvelope)
    ));
    Ok(())
}

#[test]
fn an_ingest_step_is_admitted_while_every_stage_work_unit_is_spent() -> TestResult {
    let coordinator = coordinator(12 * 1024 * 1024 * 1024, 500, 64)?;
    let mut stages = Vec::new();
    for _ in 0..10 {
        stages.push(grant(coordinator.try_reserve_profile(WORK_CLASS_STAGE))?);
    }
    check!(eq; coordinator.remaining().work, 0);
    check!(matches!(
        coordinator.try_reserve_profile(WORK_CLASS_STAGE),
        TryResult::Deferred(DepartureReason::NoWork)
    ));
    let step = grant(coordinator.try_reserve_profile(WORK_CLASS_INGEST))?;
    drop(step);
    drop(stages);
    check!(eq; coordinator.remaining().work, 1_000_000);
    check!(eq; coordinator.remaining().bytes, 12 * 1024 * 1024 * 1024);
    Ok(())
}

#[test]
fn dropped_waiter_reclaims_queue_capacity_immediately() -> TestResult {
    let coordinator = coordinator(1024, 1, 1)?;
    let held = grant(coordinator.try_reserve())?;
    drop(coordinator.wait_ticket()?);
    let next = coordinator.wait_ticket()?;
    drop(next);
    drop(held);
    check!(eq; coordinator.remainder_bytes(), 1024);
    Ok(())
}

#[tokio::test]
async fn a_blocking_job_runs_while_its_caller_holds_the_only_task_slot() -> TestResult {
    let spawner = Spawner::with_capacity(1);
    let held = grant(spawner.coordinator().try_reserve_profile(WORK_CLASS_NORMAL))?;
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        spawner.blocking(|| Ok::<_, ()>(7)),
    )
    .await
    .map_err(|_| "a blocking job nested under the last task slot never ran")?;
    check!(matches!(outcome, crate::outcome::Outcome::Ok(7)));
    drop(held);
    check!(eq; spawner.coordinator().remaining().slots.tasks, 1);
    Ok(())
}

#[tokio::test]
async fn spawner_keeps_bytes_charged_until_task_finishes() -> TestResult {
    let spawner = Spawner::with_capacity(1);
    let before = spawner.coordinator().remainder_bytes();
    let (finish, finished) = tokio::sync::oneshot::channel();
    spawner.spawn(async move {
        let _ = finished.await;
    })?;
    check!(eq; spawner.coordinator().remainder_bytes(), before - 1024);
    finish.send(()).map_err(|_| "task disappeared")?;
    spawner.drain(std::time::Duration::from_secs(5)).await?;
    check!(eq; spawner.coordinator().remainder_bytes(), before);
    Ok(())
}

#[test]
fn eligible_waiter_is_granted_without_a_departure() -> TestResult {
    let coordinator = coordinator(1024, 1, 4)?;
    let mut ticket = coordinator.wait_ticket()?;
    let granted = ticket.try_recv()??;
    drop(granted);
    check!(eq; coordinator.remainder_bytes(), 1024);
    Ok(())
}

#[test]
fn configured_default_must_fit_capacity() -> TestResult {
    check!(matches!(
        Coordinator::with(1, 10, 1, 4, WORK_CLASS_NORMAL),
        Err(DepartureReason::NoBytes)
    ));
    Ok(())
}

#[test]
fn zero_and_overflowing_configuration_is_rejected_exactly() -> TestResult {
    for (bytes, work, slots, waiters, expected) in [
        (0, 10, 1, 4, DepartureReason::NoBytes),
        (1024, 0, 1, 4, DepartureReason::NoWork),
        (1024, 10, 0, 4, DepartureReason::NoSlots),
        (1024, 10, 1, 0, DepartureReason::WaitersFull),
        (1024, 10, 1, 257, DepartureReason::WaitersFull),
        (u64::MAX, 10, 1, 4, DepartureReason::AccountingOverflow),
        (1024, u64::MAX, 1, 4, DepartureReason::AccountingOverflow),
        (1024, 10, u64::MAX, 4, DepartureReason::AccountingOverflow),
    ] {
        check!(
            matches!(Coordinator::with(bytes, work, slots, waiters, WORK_CLASS_NORMAL),
            Err(actual) if actual == expected)
        );
    }
    Ok(())
}

#[test]
fn giant_units_are_refused_without_queuing_or_charging() -> TestResult {
    let coordinator = coordinator(1024, 1, 4)?;
    for (profile, reason) in [
        (profile(1025, 1), DepartureReason::NoBytes),
        (profile(1024, 2), DepartureReason::NoSlots),
    ] {
        check!(
            matches!(coordinator.try_reserve_profile(profile), TryResult::Refused(actual) if actual == reason)
        );
        check!(matches!(coordinator.try_enqueue(profile), Err(actual) if actual == reason));
    }
    check!(eq; coordinator.remaining(), Resources { bytes: 1024, work: 1_000_000,
        slots: SlotVector { tasks: 1, stages: 1, browser: 1, populations: 1 } });
    Ok(())
}

#[test]
fn blocked_downstream_does_not_pin_the_ready_queue_head() -> TestResult {
    let coordinator = coordinator(4096, 2, 8)?;
    let stage = WorkClass::work(
        1024,
        1,
        SlotVector {
            tasks: 0,
            stages: 2,
            browser: 0,
            populations: 0,
        },
    );
    let held = grant(coordinator.try_reserve_profile(stage))?;
    let mut blocked = coordinator.try_enqueue(stage)?;
    let mut movable = coordinator.try_enqueue(profile(1024, 1))?;
    let small = movable.try_recv()??;
    check!(matches!(
        blocked.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    drop(held);
    let large = blocked.try_recv()??;
    drop((small, large));
    check!(eq; coordinator.remainder_bytes(), 4096);
    Ok(())
}

#[test]
fn repeated_small_units_cannot_starve_a_large_waiter() -> TestResult {
    let coordinator = coordinator(4096, 4, 16)?;
    let held = grant(coordinator.try_reserve_profile(profile(1024, 1)))?;
    let mut large = coordinator.try_enqueue(profile(4096, 1))?;
    for _ in 0..MAX_BYPASSES {
        drop(grant(coordinator.try_reserve_profile(profile(1024, 1)))?);
    }
    check!(matches!(
        coordinator.try_reserve_profile(profile(1024, 1)),
        TryResult::Deferred(DepartureReason::Fairness)
    ));
    drop(held);
    let reservation = large.try_recv()??;
    check!(eq; coordinator.remainder_bytes(), 0);
    drop(reservation);
    check!(eq; coordinator.remainder_bytes(), 4096);
    Ok(())
}

#[test]
fn dropping_delivered_grants_restores_all_resources() -> TestResult {
    let coordinator = coordinator(4096, 4, 8)?;
    let held = grant(coordinator.try_reserve_profile(profile(4096, 4)))?;
    let ticket = coordinator.try_enqueue(profile(2048, 2))?;
    drop(held);
    check!(eq; coordinator.remainder_bytes(), 2048);
    drop(ticket);
    check!(eq; coordinator.remainder_bytes(), 4096);
    check!(eq; coordinator.remainder_slots(), 4);
    Ok(())
}

#[test]
fn waiter_descriptors_have_global_and_class_origin_caps() -> TestResult {
    let coordinator = coordinator(1024, 1, (2 * MAX_ORIGIN_WAITERS) as u64)?;
    let held = grant(coordinator.try_reserve())?;
    let mut tickets = Vec::new();
    for _ in 0..MAX_ORIGIN_WAITERS {
        tickets.push(coordinator.wait_ticket()?);
    }
    check!(matches!(
        coordinator.wait_ticket(),
        Err(DepartureReason::OriginWaitersFull)
    ));
    let other = WorkClass {
        origin: 1,
        ..WORK_CLASS_NORMAL
    };
    for _ in 0..MAX_ORIGIN_WAITERS {
        tickets.push(coordinator.try_enqueue(other)?);
    }
    check!(matches!(
        coordinator.try_enqueue(WorkClass { origin: 2, ..other }),
        Err(DepartureReason::WaitersFull)
    ));
    drop(tickets);
    drop(held);
    check!(eq; coordinator.remainder_bytes(), 1024);
    Ok(())
}

#[test]
fn driver_lane_is_admitted_while_the_work_lane_is_protected() -> TestResult {
    let coordinator = coordinator(4096, 16, MAX_WAITERS as u64)?;
    let held = grant(coordinator.try_reserve_profile(profile(1024, 1)))?;
    let starving = coordinator.try_enqueue(profile(4096, 1))?;
    for _ in 0..MAX_BYPASSES {
        drop(grant(coordinator.try_reserve_profile(profile(1024, 1)))?);
    }
    check!(matches!(
        coordinator.try_reserve_profile(profile(1024, 1)),
        TryResult::Deferred(DepartureReason::Fairness)
    ));
    let mut waiting = vec![starving];
    for _ in 1..MAX_ORIGIN_WAITERS {
        waiting.push(coordinator.try_enqueue(profile(512, 0))?);
    }
    check!(matches!(
        coordinator.try_enqueue(profile(512, 0)),
        Err(DepartureReason::OriginWaitersFull)
    ));
    let driver = grant(coordinator.try_reserve_profile(WORK_CLASS_CONTROL))?;
    drop(driver);
    drop(waiting);
    drop(held);
    check!(eq; coordinator.remainder_bytes(), 4096);
    Ok(())
}

#[test]
fn control_and_drain_reserves_are_independent_and_bounded() -> TestResult {
    let coordinator = coordinator(1024, 1, 4)?;
    let work = grant(coordinator.try_reserve())?;
    let mut control = Vec::new();
    let mut drain = Vec::new();
    for _ in 0..64 {
        control.push(grant(coordinator.try_reserve_profile(WORK_CLASS_CONTROL))?);
        drain.push(grant(coordinator.try_reserve_profile(WORK_CLASS_DRAIN))?);
    }
    check!(matches!(
        coordinator.try_reserve_profile(WORK_CLASS_CONTROL),
        TryResult::Deferred(DepartureReason::NoBytes)
    ));
    check!(matches!(
        coordinator.try_reserve_profile(WORK_CLASS_DRAIN),
        TryResult::Deferred(DepartureReason::NoBytes)
    ));
    drop((control, drain, work));
    check!(eq; coordinator.remainder_bytes(), 1024);
    Ok(())
}

#[tokio::test]
async fn closure_is_not_constructed_before_admission() -> TestResult {
    let spawner = Spawner::with_capacity(1);
    let held = grant(spawner.coordinator().try_reserve())?;
    let constructed = std::cell::Cell::new(false);
    let outcome = spawner.spawn_with(WORK_CLASS_NORMAL, || {
        constructed.set(true);
        async {}
    });
    check!(matches!(
        outcome,
        Err(crate::spawn::SpawnError::RegionFull { capacity: 1 })
    ));
    check!(eq; constructed.get(), false);
    drop(held);
    Ok(())
}

#[test]
fn charged_transform_retains_grant_without_heap_box_or_clone() -> TestResult {
    let coordinator = coordinator(1024, 1, 4)?;
    let charged = grant(coordinator.try_reserve())?.into_charged("value");
    let next = charged.map(str::len);
    check!(eq; *next, 5);
    check!(eq; coordinator.remainder_bytes(), 0);
    drop(next);
    check!(eq; coordinator.remainder_bytes(), 1024);
    Ok(())
}

#[tokio::test]
async fn closing_wakes_waiters_without_losing_existing_grants() -> TestResult {
    let coordinator = coordinator(1024, 1, 4)?;
    let held = grant(coordinator.try_reserve())?;
    let waiting = coordinator.wait_ticket()?;
    coordinator.close();
    check!(matches!(waiting.await, Err(DepartureReason::RegionClosed)));
    check!(eq; coordinator.remainder_bytes(), 0);
    drop(held);
    check!(eq; coordinator.remainder_bytes(), 1024);
    check!(matches!(
        coordinator.try_reserve(),
        TryResult::Refused(DepartureReason::RegionClosed)
    ));
    Ok(())
}
