use super::{
    io::{RecoveryFsOperation, RecoveryPayloadRole},
    PreparedRecoveryBudget,
};
use crate::acquisition::budget::target::limits;
use crate::acquisition::{AcquisitionSelectionUsage, CaptureError};
use crate::{StoreError, StoreResult};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod allocations;
mod diagnostics;
mod phases;
mod traversal;

fn usage(count: u64) -> StoreResult<AcquisitionSelectionUsage> {
    let roots = count.checked_mul(2).ok_or(StoreError::CounterOverflow)?;
    AcquisitionSelectionUsage::new(roots, roots, 0, 0, count, 0)
}
fn exceeded<T>(result: StoreResult<T>, expected: &'static str) {
    assert!(matches!(result,
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded { resource })) if resource == expected));
}
fn refused<T>(result: StoreResult<T>) {
    assert!(matches!(
        result,
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
}

#[test]
fn prepared_recovery_plan_twelve_admits_thirteen_refuses_before_targets() -> TestResult {
    let budget = PreparedRecoveryBudget::admit(usage(12)?, 12)?;
    check!(eq; budget.targets.limit, 12);
    check!(eq; budget.targets.used, 0);
    check!(eq; budget.shell.payload.limit, 2_017_198_080);
    check!(eq; budget.shell.payload.used, 0);
    check!(eq; budget.reads.recovery_totals().roots.limit, 504);
    check!(eq; budget.reads.recovery_totals().encoded.limit, 37_472_256);
    check!(eq; budget.reads.recovery_totals().receipts.limit, 36);
    exceeded(
        PreparedRecoveryBudget::admit(usage(13)?, 13),
        "backing bytes",
    );
    Ok(())
}

#[test]
fn prepared_recovery_plan_remaining_roots_encoded_n_and_n_minus_one() -> TestResult {
    let exact = AcquisitionSelectionUsage::new(1_048_534, 533_748_224, 0, 0, 25_000, 0)?;
    let mut budget = PreparedRecoveryBudget::admit(exact, 1)?;
    {
        let mut account = budget.begin_target()?;
        for _ in 0..42 {
            account.reads().row(4_096, 65_536)?;
        }
        for _ in 0..3 {
            account.reads().receipt_probe()?;
            account.reads().encoded(66_048)?;
        }
        check!(eq; account.reads().selection_usage().roots(), 42);
        check!(eq;
            account.reads().selection_usage().encoded_bytes(),
            limits::ENCODED
        );
        exceeded(account.reads().row(1, 1), "roots");
        exceeded(account.reads().encoded(1), "encoded bytes");
    }
    let roots_short = AcquisitionSelectionUsage::new(1_048_535, 533_748_224, 0, 0, 25_000, 0)?;
    exceeded(PreparedRecoveryBudget::admit(roots_short, 1), "roots");
    let encoded_short = AcquisitionSelectionUsage::new(1_048_534, 533_748_225, 0, 0, 25_000, 0)?;
    exceeded(
        PreparedRecoveryBudget::admit(encoded_short, 1),
        "encoded bytes",
    );
    Ok(())
}

#[test]
fn prepared_recovery_actual_row_and_receipt_debits_cannot_reset() -> TestResult {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    for _ in 0..2 {
        let mut account = budget.begin_target()?;
        for _ in 0..42 {
            account.reads().row(1, 1)?;
        }
        for _ in 0..3 {
            account.reads().receipt_probe()?;
        }
        account.reads().begin_operation();
        exceeded(account.reads().row(1, 1), "roots");
        exceeded(account.reads().receipt_probe(), "receipt probes");
    }
    check!(eq; budget.reads.recovery_totals().roots.used, 84);
    check!(eq; budget.reads.recovery_totals().receipts.used, 6);
    check!(eq; budget.reads.selection_usage().encoded_bytes(), 168);
    exceeded(budget.begin_target(), "recovery targets");
    Ok(())
}

#[test]
fn prepared_recovery_zero_pin_admission_has_no_new_target() -> TestResult {
    let mut budget =
        PreparedRecoveryBudget::admit(AcquisitionSelectionUsage::new(0, 0, 0, 0, 3, 3)?, 0)?;
    check!(eq; budget.shell.payload.limit, 0);
    check!(eq; budget.reads.recovery_totals().allocations.owned.limit, 0);
    exceeded(budget.begin_target(), "recovery targets");
    exceeded(
        PreparedRecoveryBudget::admit(usage(1)?, 2),
        "selection usage",
    );
    Ok(())
}

#[test]
fn prepared_recovery_selection_usage_rejects_incoherent_six_scalars() {
    for args in [
        [0, 0, 0, 0, 1, 0],
        [1, 1, 0, 0, 1, 1],
        [0, 1, 0, 0, 1, 1],
        [0, 0, 1, 0, 1, 1],
        [2, 0, 0, 0, 1, 0],
        [43, 1, 0, 0, 1, 0],
        [2, 139_265, 0, 0, 1, 0],
        [2, 2, 4, 0, 1, 0],
        [2, 2, 1, 2, 1, 0],
        [2, 2, 0, 0, 1, 2],
    ] {
        let [roots, encoded, probes, present, targets, absent] = args;
        exceeded(
            AcquisitionSelectionUsage::new(roots, encoded, probes, present, targets, absent),
            "selection usage",
        );
    }
}

#[test]
fn prepared_recovery_parent_backing_threshold_is_exact() -> TestResult {
    let mut exact = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    exact.shell.payload.limit = limits::PAYLOAD;
    {
        let mut account = exact.begin_target()?;
        account.payload(RecoveryPayloadRole::Body, 100_663_296)?;
        account.payload(RecoveryPayloadRole::Metadata, 196_608)?;
        account.start_publication()?;
        account.payload(RecoveryPayloadRole::Body, 67_108_864)?;
        account.payload(RecoveryPayloadRole::Metadata, 131_072)?;
    }
    check!(eq; exact.shell.payload.used, limits::PAYLOAD);
    let mut short = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    short.shell.payload.limit = limits::PAYLOAD
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    let mut account = short.begin_target()?;
    account.payload(RecoveryPayloadRole::Body, 100_663_296)?;
    account.payload(RecoveryPayloadRole::Metadata, 196_608)?;
    exceeded(account.start_publication(), "publication payload");
    refused(account.start_publication());
    Ok(())
}

#[test]
fn prepared_recovery_checked_counter_product_and_addition_overflow_refuse() {
    assert!(matches!(
        crate::acquisition::budget::target::counter::product(u64::MAX, 2),
        Err(StoreError::CounterOverflow)
    ));
    let full = crate::acquisition::budget::target::counter::Counter {
        limit: u64::MAX,
        used: u64::MAX,
    };
    assert!(matches!(
        full.charged(1, "directory events"),
        Err(StoreError::CounterOverflow)
    ));
    assert!(matches!(
        AcquisitionSelectionUsage::new(u64::MAX, 1, 0, 0, 1, 0),
        Err(StoreError::CounterOverflow)
    ));
}
