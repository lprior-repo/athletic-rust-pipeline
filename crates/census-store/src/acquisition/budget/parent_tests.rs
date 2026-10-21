use super::AcquisitionReadBudget;
use crate::{CaptureError, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn recovery() -> StoreResult<AcquisitionReadBudget> {
    let mut budget = AcquisitionReadBudget::for_recovery(42, 3_122_688, 1)?;
    budget.begin_recovery_target()?;
    Ok(budget)
}

fn parent_reads(budget: &mut AcquisitionReadBudget) -> StoreResult<()> {
    budget.public_parent(|parent| {
        for _ in 0..42 {
            parent.row(2, 3)?;
        }
        parent.encoded(7)?;
        parent.public_owner_record(11, 13)?;
        for _ in 0..3 {
            parent.receipt_probe()?;
        }
        for _ in 0..2 {
            parent.present_receipt()?;
        }
        Ok(())
    })
}

fn exceeded<T>(
    result: StoreResult<T>,
    resource: &'static str,
    required: u64,
    limit: u64,
) -> TestResult {
    match result {
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: actual,
            required: actual_required,
            limit: actual_limit,
        })) => {
            check!(eq; actual, resource);
            check!(eq; actual_required, required);
            check!(eq; actual_limit, limit);
            Ok(())
        }
        _ => Err("parent read did not enforce the exact expected budget".into()),
    }
}

#[test]
fn selection_parent_reads_preserve_closure_and_separate_receipt_telemetry() -> TestResult {
    let mut budget = AcquisitionReadBudget::for_selection();
    let closure = budget.selection_usage();
    parent_reads(&mut budget)?;
    check!(eq; budget.selection_usage(), closure);
    let parent = budget.public_parent_usage();
    check!(eq; (parent.rows(), parent.encoded_bytes()), (43, 241));
    check!(eq; (parent.receipt_probes(), parent.present_receipts()), (3, 2));
    let totals = budget.recovery_totals();
    check!(eq; totals.admission.limit, 219_581_775_872);
    check!(eq; totals.admission.used, 241);
    check!(eq; totals.admission_probes.used, 3);
    check!(eq; (totals.roots.used, totals.encoded.used, totals.receipts.used), (0, 0, 0));
    budget.target_examined()?;
    for _ in 0..42 {
        budget.row(1, 2)?;
    }
    for _ in 0..3 {
        budget.receipt_probe()?;
    }
    for _ in 0..2 {
        budget.present_receipt()?;
    }
    let child = budget.selection_usage();
    check!(eq; (child.roots(), child.encoded_bytes()), (42, 126));
    check!(eq; (child.receipt_probes(), child.present_receipts()), (3, 2));
    check!(eq; child.targets_examined(), 1);
    check!(eq; budget.public_parent_usage(), parent);
    let mut admitted = crate::PreparedRecoveryBudget::admit(child, 1)?;
    check!(eq; admitted.reads().recovery_totals().roots.limit, 42);
    check!(eq; admitted.reads().recovery_totals().receipts.limit, 3);
    Ok(())
}

#[test]
fn recovery_parent_reads_leave_all_40_or_42_roots_and_three_child_receipts() -> TestResult {
    for (roots, bytes) in [(40, 80), (42, 84)] {
        let mut budget = recovery()?;
        parent_reads(&mut budget)?;
        for _ in 0..roots {
            budget.row(1, 1)?;
        }
        for _ in 0..3 {
            budget.receipt_probe()?;
            budget.present_receipt()?;
        }
        let child = budget.selection_usage();
        check!(eq; (child.roots(), child.encoded_bytes()), (roots, bytes));
        check!(eq; (child.receipt_probes(), child.present_receipts()), (3, 3));
        let totals = budget.recovery_totals();
        check!(eq; (totals.roots.limit, totals.encoded.limit, totals.receipts.limit), (42, 3_122_688, 3));
        check!(eq; totals.admission.limit, 3_350_552);
        check!(eq; totals.admission.used, 241);
        exceeded(budget.receipt_probe(), "receipt probes", 4, 3)?;
        for _ in roots..42 {
            budget.row(1, 1)?;
        }
        exceeded(budget.row(0, 1), "roots", 1, 0)?;
        check!(eq; budget.recovery_totals().roots.used, 42);
        check!(eq; budget.public_parent_usage().rows(), 43);
        check!(eq; budget.public_parent_usage().present_receipts(), 2);
    }
    Ok(())
}

#[test]
fn parent_and_owner_reads_share_the_exact_existing_admission_capacity() -> TestResult {
    let recovery = recovery()?;
    for (mut budget, cap) in [
        (AcquisitionReadBudget::for_selection(), 219_581_775_872_u64),
        (recovery, 3_350_552_u64),
    ] {
        let debit = cap.checked_sub(12).ok_or("admission prefix")?;
        budget.public_owner_record(0, usize::try_from(debit)?)?;
        budget.public_parent(|parent| {
            parent.row(2, 3)?;
            parent.encoded(4)?;
            parent.public_owner_record(1, 2)
        })?;
        check!(eq; budget.recovery_totals().admission.used, cap);
        check!(eq; budget.recovery_totals().admission.limit, cap);
        let one_over = cap.checked_add(1).ok_or("admission overflow")?;
        exceeded(
            budget.public_parent(|parent| parent.encoded(1)),
            "admission encoded bytes",
            one_over,
            cap,
        )?;
        exceeded(
            budget.public_owner_record(0, 1),
            "admission encoded bytes",
            one_over,
            cap,
        )?;
        check!(eq; budget.recovery_totals().admission.used, cap);
        check!(eq; budget.selection_usage().roots(), 0);
    }
    Ok(())
}

#[test]
fn one_byte_short_parent_admission_refuses_without_resetting_consumed_credit() -> TestResult {
    let recovery = recovery()?;
    for (mut budget, cap) in [
        (AcquisitionReadBudget::for_selection(), 219_581_775_872_u64),
        (recovery, 3_350_552_u64),
    ] {
        let debit = cap.checked_sub(5).ok_or("admission prefix")?;
        budget.admission_value(0, usize::try_from(debit)?)?;
        let one_over = cap.checked_add(1).ok_or("admission overflow")?;
        exceeded(
            budget.public_parent(|parent| parent.row(2, 4)),
            "admission encoded bytes",
            one_over,
            cap,
        )?;
        check!(eq; budget.recovery_totals().admission.used, debit);
        budget.public_parent(|parent| parent.encoded(5))?;
        check!(eq; budget.recovery_totals().admission.used, cap);
        budget.row(1, 1)?;
        check!(eq; budget.selection_usage().roots(), 1);
        check!(eq; budget.selection_usage().encoded_bytes(), 2);
        check!(eq; budget.recovery_totals().admission.used, cap);
    }
    Ok(())
}

#[test]
fn ordinary_parent_reads_still_obey_normal_row_byte_and_receipt_accounting() -> TestResult {
    for (roots, resource) in [(1, "roots"), (2, "encoded bytes")] {
        let mut budget = AcquisitionReadBudget::new(roots, 5, 0)?;
        budget.public_parent(|parent| {
            parent.row(2, 3)?;
            parent.receipt_probe()?;
            parent.present_receipt()
        })?;
        let closure = budget.selection_usage();
        check!(eq; (closure.roots(), closure.encoded_bytes()), (1, 5));
        check!(eq; (closure.receipt_probes(), closure.present_receipts()), (1, 1));
        let parent = budget.public_parent_usage();
        check!(eq; (parent.rows(), parent.encoded_bytes()), (1, 5));
        check!(eq; (parent.receipt_probes(), parent.present_receipts()), (1, 1));
        exceeded(
            budget.public_parent(|parent| parent.row(0, 1)),
            resource,
            1,
            0,
        )?;
        exceeded(
            budget.public_parent(|parent| parent.encoded(1)),
            "encoded bytes",
            1,
            0,
        )?;
        check!(eq; budget.selection_usage(), closure);
        check!(eq; budget.recovery_totals().admission.used, 0);
    }
    Ok(())
}

#[test]
fn parent_error_restores_normal_routing_and_keeps_prior_debits() -> TestResult {
    let mut budget = recovery()?;
    let result: StoreResult<()> = budget.public_parent(|parent| {
        parent.row(2, 3)?;
        parent.encoded(7)?;
        Err(CaptureError::BackingIntegrity.into())
    });
    check!(matches!(
        result,
        Err(StoreError::Capture(CaptureError::BackingIntegrity))
    ));
    check!(eq; budget.recovery_totals().admission.used, 12);
    budget.row(1, 1)?;
    check!(eq; budget.selection_usage().roots(), 1);
    check!(eq; budget.selection_usage().encoded_bytes(), 2);
    budget.public_parent(|parent| parent.public_owner_record(3, 4))?;
    check!(eq; budget.recovery_totals().admission.used, 19);
    check!(eq; budget.public_parent_usage().rows(), 2);
    check!(eq; budget.public_parent_usage().encoded_bytes(), 19);
    check!(eq; budget.selection_usage().roots(), 1);
    Ok(())
}

#[test]
fn nested_parent_mode_refuses_without_running_or_clearing_outer_routing() -> TestResult {
    let mut budget = recovery()?;
    let mut invoked = false;
    let nested = budget.public_parent(|parent| {
        parent.row(1, 1)?;
        let nested = parent.public_parent(|_| {
            invoked = true;
            Ok(())
        });
        parent.encoded(3)?;
        Ok(nested)
    })?;
    check!(!invoked);
    check!(matches!(
        nested,
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(eq; budget.recovery_totals().admission.used, 5);
    check!(eq; budget.selection_usage().roots(), 0);
    budget.row(1, 1)?;
    check!(eq; budget.selection_usage().roots(), 1);
    budget.public_parent(|parent| parent.encoded(1))?;
    check!(eq; budget.recovery_totals().admission.used, 6);
    Ok(())
}
