use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn coordinator(waiters: u64) -> TestResult<Coordinator> {
    Ok(Coordinator::with(
        1024 * 1024 * 1024,
        1_000_000,
        8,
        waiters,
        WORK_CLASS_NORMAL,
    )?)
}

fn grant(result: TryResult<Reservation>) -> TestResult<Reservation> {
    match result {
        TryResult::Granted(held) => Ok(held),
        TryResult::Deferred(reason) | TryResult::Refused(reason) => Err(reason.into()),
    }
}

#[test]
fn publication_charges_one_shared_population_without_spending_control_reserves() -> TestResult {
    let coordinator = coordinator(8)?;
    let before = coordinator.remaining();
    check!(eq; before.slots.populations, 1);
    check!(eq; WORK_CLASS_PUBLICATION.class, Class::Work);
    check!(eq; WORK_CLASS_PUBLICATION.resources.bytes, WORK_CLASS_STAGE.resources.bytes);
    check!(eq; WORK_CLASS_PUBLICATION.resources.work, WORK_CLASS_STAGE.resources.work);
    let held = grant(coordinator.try_reserve_profile(WORK_CLASS_PUBLICATION))?;
    let charged = before.checked_sub(WORK_CLASS_PUBLICATION.resources)?;
    check!(eq; coordinator.remaining(), charged);
    check!(charged.slots.stages > 0 && charged.bytes > 0 && charged.work > 0);
    for origin in [0, 1, u64::MAX] {
        check!(matches!(
            coordinator.try_reserve_profile(WorkClass {
                origin,
                ..WORK_CLASS_PUBLICATION
            }),
            TryResult::Deferred(DepartureReason::NoSlots)
        ));
        check!(eq; coordinator.remaining(), charged);
    }
    let control = grant(coordinator.try_reserve_profile(WORK_CLASS_CONTROL))?;
    let drain = grant(coordinator.try_reserve_profile(WORK_CLASS_DRAIN))?;
    check!(eq; coordinator.remaining(), charged);
    drop((control, drain, held));
    check!(eq; coordinator.remaining(), before);
    Ok(())
}

#[test]
fn population_dimension_is_validated_and_checked_without_other_units() -> TestResult {
    let coordinator = coordinator(8)?;
    let before = coordinator.remaining();
    let only_population = WorkClass::work(
        0,
        0,
        SlotVector {
            populations: 1,
            ..SlotVector::default()
        },
    );
    let held = grant(coordinator.try_reserve_profile(only_population))?;
    check!(eq; coordinator.remaining(), before.checked_sub(only_population.resources)?);
    check!(matches!(
        Resources::default().checked_sub(only_population.resources),
        Err(DepartureReason::AccountingOverflow)
    ));
    let full = Resources {
        slots: SlotVector {
            populations: u64::MAX,
            ..SlotVector::default()
        },
        ..Resources::default()
    };
    check!(matches!(
        full.checked_add(only_population.resources),
        Err(DepartureReason::AccountingOverflow)
    ));
    drop(held);
    check!(eq; coordinator.remaining(), before);
    Ok(())
}

#[test]
fn oversized_population_and_reserved_pool_requests_are_refused_without_charge() -> TestResult {
    let coordinator = coordinator(8)?;
    let before = coordinator.remaining();
    for class in [Class::Work, Class::Control, Class::Drain] {
        let request = WorkClass {
            class,
            resources: Resources {
                slots: SlotVector {
                    populations: if class == Class::Work { 2 } else { 1 },
                    ..SlotVector::default()
                },
                ..Resources::default()
            },
            origin: 0,
        };
        check!(matches!(
            coordinator.try_reserve_profile(request),
            TryResult::Refused(DepartureReason::NoSlots)
        ));
        check!(eq; coordinator.remaining(), before);
    }
    Ok(())
}

#[test]
fn publication_waiter_cancellation_and_granted_ticket_drop_restore_exact_capacity() -> TestResult {
    let coordinator = coordinator(2)?;
    let before = coordinator.remaining();
    let held = grant(coordinator.try_reserve_profile(WORK_CLASS_PUBLICATION))?;
    let first = coordinator.try_enqueue(WORK_CLASS_PUBLICATION)?;
    let mut second = coordinator.try_enqueue(WORK_CLASS_PUBLICATION)?;
    check!(matches!(
        coordinator.try_enqueue(WORK_CLASS_PUBLICATION),
        Err(DepartureReason::WaitersFull)
    ));
    drop(first);
    let third = coordinator.try_enqueue(WORK_CLASS_PUBLICATION)?;
    drop(held);
    let granted = second.try_recv()??;
    check!(eq; coordinator.remaining(), before.checked_sub(WORK_CLASS_PUBLICATION.resources)?);
    drop(granted);
    check!(eq; coordinator.remaining(), before.checked_sub(WORK_CLASS_PUBLICATION.resources)?);
    drop(third);
    check!(eq; coordinator.remaining(), before);
    Ok(())
}

#[test]
fn closing_admission_refuses_waiters_but_keeps_the_running_population_charged() -> TestResult {
    let coordinator = coordinator(8)?;
    let before = coordinator.remaining();
    let held = grant(coordinator.try_reserve_profile(WORK_CLASS_PUBLICATION))?;
    let mut waiting = coordinator.try_enqueue(WORK_CLASS_PUBLICATION)?;
    coordinator.close();
    check!(matches!(
        waiting.try_recv()?,
        Err(DepartureReason::RegionClosed)
    ));
    check!(matches!(
        coordinator.try_reserve_profile(WORK_CLASS_PUBLICATION),
        TryResult::Refused(DepartureReason::RegionClosed)
    ));
    check!(eq; coordinator.remaining(), before.checked_sub(WORK_CLASS_PUBLICATION.resources)?);
    drop(held);
    check!(eq; coordinator.remaining(), before);
    Ok(())
}

#[test]
fn stage_configuration_cannot_expand_or_reset_population_capacity() -> TestResult {
    let coordinator = coordinator(8)?;
    let held = grant(coordinator.try_reserve_profile(WORK_CLASS_PUBLICATION))?;
    let charged = coordinator.remaining();
    check!(matches!(
        coordinator.configure_stages(32),
        Err(DepartureReason::ConfigurationInUse)
    ));
    check!(eq; coordinator.remaining(), charged);
    drop(held);
    coordinator.configure_stages(32)?;
    check!(eq; coordinator.remaining().slots.stages, 32);
    check!(eq; coordinator.remaining().slots.populations, 1);
    Ok(())
}
