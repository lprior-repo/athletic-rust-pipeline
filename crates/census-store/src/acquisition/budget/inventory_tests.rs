use super::AcquisitionReadBudget;
use crate::acquisition::AcquisitionSelectionUsage;
use crate::{CaptureError, PreparedRecoveryBudget, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn cold_inventory_encoded_reads_cross_old_ceiling_and_exhaust_at_eight_gib() -> TestResult {
    let mut budget = AcquisitionReadBudget::cold_inventory()?;
    budget.encoded(536_870_912)?;
    budget.begin_operation();
    budget.encoded(1)?;
    budget.encoded(usize::try_from(8_053_063_679_u64)?)?;
    match budget.encoded(1) {
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "encoded bytes",
            required: 1,
            limit: 0,
        })) => {}
        _ => return Err("cold inventory did not enforce eight GiB without refund".into()),
    }
    check!(eq; budget.roots, 1_048_576);
    check!(eq; budget.backing, 34_359_738_368);
    check!(eq; budget.retained, 67_108_864);
    check!(eq; budget.encoded, 0);
    Ok(())
}

fn exhausted(result: StoreResult<()>) -> TestResult {
    match result {
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "backing bytes",
            required: 1,
            limit: 0,
        })) => Ok(()),
        _ => Err("backing budget did not enforce its exact remaining allowance".into()),
    }
}

#[test]
fn inventory_and_selection_backing_charge_cumulatively_to_thirty_two_gib() -> TestResult {
    for mut budget in [
        AcquisitionReadBudget::inventory(),
        AcquisitionReadBudget::for_selection(),
    ] {
        check!(eq; budget.roots, 1_048_576);
        check!(eq; budget.encoded, 536_870_912);
        check!(eq; budget.retained, 67_108_864);
        check!(eq; budget.backing, 34_359_738_368);
        budget.backing(32_212_254_720)?;
        budget.begin_operation();
        check!(eq; budget.backing, 2_147_483_648);
        budget.backing(2_147_483_648)?;
        exhausted(budget.backing(1))?;
        check!(eq; budget.backing, 0);
    }
    Ok(())
}

#[test]
fn public_constructor_keeps_exact_two_gib_and_refuses_one_byte_more() -> TestResult {
    let mut budget = AcquisitionReadBudget::new(1, 1, 2_147_483_648)?;
    budget.backing(2_147_483_648)?;
    exhausted(budget.backing(1))?;
    match AcquisitionReadBudget::new(1, 1, 2_147_483_649) {
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "configuration",
        })) => Ok(()),
        _ => Err("public acquisition budget accepted backing above two GiB".into()),
    }
}

#[test]
fn per_work_backing_keeps_its_exact_allowance_without_operation_refund() -> TestResult {
    let mut budget = AcquisitionReadBudget::for_work();
    check!(eq; budget.roots, 84);
    check!(eq; budget.backing, 33_619_968);
    budget.backing(33_619_968)?;
    budget.begin_operation();
    exhausted(budget.backing(1))?;
    check!(eq; budget.backing, 0);
    Ok(())
}

#[test]
fn prepared_recovery_keeps_its_separate_two_gib_payload_admission() -> TestResult {
    let usage = AcquisitionSelectionUsage::new(26, 26, 0, 0, 13, 0)?;
    match PreparedRecoveryBudget::admit(usage, 13) {
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "backing bytes",
            required: 2_185_297_920,
            limit: 2_147_483_648,
        })) => Ok(()),
        _ => Err("prepared recovery changed its separate backing admission ceiling".into()),
    }
}
