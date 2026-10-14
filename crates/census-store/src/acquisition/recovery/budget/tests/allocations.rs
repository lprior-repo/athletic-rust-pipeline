use super::*;

#[test]
fn prepared_recovery_allocation_live_release_never_restores_cumulative_credit() -> TestResult {
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
    check!(eq; budget.reads.recovery_totals().allocations.live_owned, 0);
    check!(eq;
        budget.reads.recovery_totals().allocations.owned.used,
        limits::NONBODY
    );
    {
        let mut account = budget.begin_target()?;
        exceeded(account.reserve_owned(67_108_864), "allocation");
        account.reserve_owned(67_108_863)?;
    }
    let allocations = budget.reads.recovery_totals().allocations;
    check!(eq; allocations.owned.used, cap);
    check!(eq; allocations.live_owned, 0);
    check!(eq; allocations.peak, limits::NONBODY);
    check!(eq; allocations.calls.used, 2);
    Ok(())
}

#[test]
fn prepared_recovery_body_scratch_are_once_per_target_with_one_live_bulk_pair() -> TestResult {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    for _ in 0..2 {
        let mut account = budget.begin_target()?;
        account.reserve_body(limits::BODY)?;
        account.reserve_scratch()?;
        refused(account.reserve_body(1));
        refused(account.reserve_scratch());
        check!(eq;
            account.reads().recovery_totals().allocations.live()?,
            33_587_200
        );
    }
    let allocations = budget.reads.recovery_totals().allocations;
    check!(eq; allocations.body.used, 67_108_864);
    check!(eq; allocations.scratch.used, 65_536);
    check!(eq; allocations.calls.used, 4);
    check!(eq; allocations.live()?, 0);
    check!(eq; allocations.peak, 33_587_200);
    Ok(())
}

#[test]
fn prepared_recovery_allocation_calls_n_minus_one_refuses_before_reservation() -> TestResult {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    budget.reads.recovery_limit_calls(1);
    let mut account = budget.begin_target()?;
    account.reserve_owned(10)?;
    exceeded(account.reserve_owned(1), "allocation calls");
    check!(eq; account.reads().recovery_totals().allocations.owned.used, 10);
    check!(eq; account.reads().recovery_totals().allocations.calls.used, 1);
    account.release_owned(10)?;
    exceeded(account.release_owned(1), "live allocation");
    check!(eq; account.reads().recovery_totals().allocations.live_owned, 0);
    Ok(())
}

#[test]
fn prepared_recovery_admission_encoded_n_and_n_minus_one() -> TestResult {
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
    check!(eq; totals.admission.used, 3_350_552);
    check!(eq; totals.admission_probes.used, 20);
    check!(eq; totals.encoded.used, 0);
    check!(eq; totals.roots.used, 0);
    Ok(())
}

#[test]
fn prepared_recovery_empty_body_uses_no_body_allocation_and_oversized_body_refuses() -> TestResult {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    {
        let mut account = budget.begin_target()?;
        account.reserve_body(0)?;
        refused(account.reserve_body(1));
    }
    check!(eq; budget.reads.recovery_totals().allocations.body.used, 0);
    check!(eq; budget.reads.recovery_totals().allocations.calls.used, 0);
    {
        let mut account = budget.begin_target()?;
        exceeded(account.reserve_body(33_554_433), "body allocation");
        check!(eq; account.reads().recovery_totals().allocations.body.used, 0);
        account.reserve_body(limits::BODY)?;
    }
    check!(eq;
        budget.reads.recovery_totals().allocations.body.used,
        limits::BODY
    );
    check!(eq; budget.reads.recovery_totals().allocations.live_body, 0);
    Ok(())
}

#[test]
fn prepared_recovery_box_debits_survive_release_and_refuse_n_minus_one_calls() -> TestResult {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    budget.reads.recovery_limit_calls(2);
    {
        let mut account = budget.begin_target()?;
        account.reserve_owned(256)?;
        account.release_owned(256)?;
        account.reserve_owned(256)?;
        account.release_owned(256)?;
        exceeded(account.reserve_owned(256), "allocation calls");
        let totals = account.reads().recovery_totals().allocations;
        check!(eq; totals.owned.used, 512);
        check!(eq; totals.calls.used, 2);
        check!(eq; totals.live_owned, 0);
    }
    check!(eq; budget.reads.recovery_totals().allocations.owned.used, 512);
    check!(eq; budget.reads.recovery_totals().allocations.calls.used, 2);
    check!(eq; budget.reads.recovery_totals().allocations.live_owned, 0);
    Ok(())
}
