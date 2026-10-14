use super::*;

#[test]
fn prepared_recovery_allocation_live_release_never_restores_cumulative_credit() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    let cap = limits::NONBODY
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_sub(1))
        .ok_or(StoreError::CounterOverflow)?;
    budget.reads.recovery_limit_owned(cap);
    {
        let mut account = budget.begin_target()?;
        account.reserve_owned(67_108_864)?;
        account.release_owned(67_108_864)?;
        exceeded(account.reserve_owned(1), "allocation");
        account.reads().begin_operation();
        exceeded(account.reserve_owned(1), "allocation");
    }
    assert_eq!(budget.reads.recovery_totals().allocations.live_owned, 0);
    assert_eq!(
        budget.reads.recovery_totals().allocations.owned.used,
        limits::NONBODY
    );
    {
        let mut account = budget.begin_target()?;
        exceeded(account.reserve_owned(67_108_864), "allocation");
        account.reserve_owned(67_108_863)?;
    }
    let allocations = budget.reads.recovery_totals().allocations;
    assert_eq!(allocations.owned.used, cap);
    assert_eq!(allocations.live_owned, 0);
    assert_eq!(allocations.peak, limits::NONBODY);
    assert_eq!(allocations.calls.used, 2);
    Ok(())
}

#[test]
fn prepared_recovery_body_scratch_are_once_per_target_with_one_live_bulk_pair() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    for _ in 0..2 {
        let mut account = budget.begin_target()?;
        account.reserve_body(limits::BODY)?;
        account.reserve_scratch()?;
        refused(account.reserve_body(1));
        refused(account.reserve_scratch());
        assert_eq!(
            account.reads().recovery_totals().allocations.live()?,
            33_587_200
        );
    }
    let allocations = budget.reads.recovery_totals().allocations;
    assert_eq!(allocations.body.used, 67_108_864);
    assert_eq!(allocations.scratch.used, 65_536);
    assert_eq!(allocations.calls.used, 4);
    assert_eq!(allocations.live()?, 0);
    assert_eq!(allocations.peak, 33_587_200);
    Ok(())
}

#[test]
fn prepared_recovery_allocation_calls_n_minus_one_refuses_before_reservation() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    budget.reads.recovery_limit_calls(1);
    let mut account = budget.begin_target()?;
    account.reserve_owned(10)?;
    exceeded(account.reserve_owned(1), "allocation calls");
    assert_eq!(account.reads().recovery_totals().allocations.owned.used, 10);
    assert_eq!(account.reads().recovery_totals().allocations.calls.used, 1);
    account.release_owned(10)?;
    exceeded(account.release_owned(1), "live allocation");
    assert_eq!(account.reads().recovery_totals().allocations.live_owned, 0);
    Ok(())
}

#[test]
fn prepared_recovery_admission_encoded_n_and_n_minus_one() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    let mut account = budget.begin_target()?;
    account.reads().admission_value(0, 3_350_551)?;
    account.reads().admission_value(0, 1)?;
    exceeded(
        account.reads().admission_value(0, 1),
        "admission encoded bytes",
    );
    for _ in 0..20 {
        account.reads().admission_probe()?;
    }
    let totals = account.reads().recovery_totals();
    assert_eq!(totals.admission.used, 3_350_552);
    assert_eq!(totals.admission_probes.used, 20);
    assert_eq!(totals.encoded.used, 0);
    assert_eq!(totals.roots.used, 0);
    Ok(())
}

#[test]
fn prepared_recovery_empty_body_uses_no_body_allocation_and_oversized_body_refuses(
) -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    {
        let mut account = budget.begin_target()?;
        account.reserve_body(0)?;
        refused(account.reserve_body(1));
    }
    assert_eq!(budget.reads.recovery_totals().allocations.body.used, 0);
    assert_eq!(budget.reads.recovery_totals().allocations.calls.used, 0);
    {
        let mut account = budget.begin_target()?;
        exceeded(account.reserve_body(33_554_433), "body allocation");
        assert_eq!(account.reads().recovery_totals().allocations.body.used, 0);
        account.reserve_body(limits::BODY)?;
    }
    assert_eq!(
        budget.reads.recovery_totals().allocations.body.used,
        limits::BODY
    );
    assert_eq!(budget.reads.recovery_totals().allocations.live_body, 0);
    Ok(())
}
