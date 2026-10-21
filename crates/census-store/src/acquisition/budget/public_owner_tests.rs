use super::AcquisitionReadBudget;
use crate::{CaptureError, StoreError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selection_public_owner_reads_consume_exact_admission_capacity_without_closure_roots(
) -> TestResult {
    let mut budget = AcquisitionReadBudget::for_selection();
    let limit = budget.recovery_totals().admission.limit;
    check!(eq; limit, 219_581_775_872);
    let debit = limit.checked_sub(5).ok_or("remaining admission")?;
    budget.admission_value(0, usize::try_from(debit)?)?;
    let closure = budget.selection_usage();
    budget.public_owner_record(2, 3)?;
    check!(eq; budget.recovery_totals().admission.used, limit);
    check!(eq; budget.selection_usage(), closure);
    let one_over = limit
        .checked_add(1)
        .ok_or("one admission byte beyond cap")?;
    match budget.public_owner_record(0, 1) {
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "admission encoded bytes",
            required,
            limit: actual,
        })) => {
            check!(eq; required, one_over);
            check!(eq; actual, limit);
        }
        _ => return Err("selection admission byte limit was not enforced".into()),
    }
    check!(eq; budget.recovery_totals().admission.used, limit);
    check!(eq; budget.selection_usage(), closure);
    Ok(())
}

#[test]
fn recovery_public_owner_reads_preserve_the_existing_separate_admission_charge() -> TestResult {
    let mut budget = AcquisitionReadBudget::for_recovery(42, 3_122_688, 1)?;
    budget.begin_recovery_target()?;
    let closure = budget.selection_usage();
    budget.public_owner_record(2, 3)?;
    let totals = budget.recovery_totals();
    check!(eq; totals.admission.limit, 3_350_552);
    check!(eq; totals.admission.used, 5);
    check!(eq; totals.roots.limit, 42);
    check!(eq; totals.roots.used, 0);
    check!(eq; totals.encoded.used, 0);
    check!(eq; budget.selection_usage(), closure);
    Ok(())
}

#[test]
fn ordinary_public_owner_reads_keep_both_physical_row_and_byte_limits() -> TestResult {
    for (roots, resource) in [(1, "roots"), (2, "encoded bytes")] {
        let mut budget = AcquisitionReadBudget::new(roots, 5, 0)?;
        budget.public_owner_record(2, 3)?;
        let closure = budget.selection_usage();
        check!(eq; (closure.roots(), closure.encoded_bytes()), (1, 5));
        match budget.public_owner_record(0, 1) {
            Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
                resource: actual,
                required: 1,
                limit: 0,
            })) => check!(eq; actual, resource),
            _ => return Err("ordinary public owner limit was not enforced".into()),
        }
        check!(eq; budget.selection_usage(), closure);
        check!(eq; budget.recovery_totals().admission.used, 0);
    }
    Ok(())
}
