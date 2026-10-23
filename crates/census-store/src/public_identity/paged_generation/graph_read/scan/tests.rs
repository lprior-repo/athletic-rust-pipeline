use super::preflight;
use crate::public_identity::authority::graph::BUDGET;
use crate::public_identity::paged_generation::{wire, InventoryV2};
use crate::{ReadBudget, StoreError, StoreResult};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn inventory(rows: u64, raw_bytes: u64) -> StoreResult<InventoryV2> {
    InventoryV2::from_wire(wire::Inventory {
        rows,
        raw_bytes,
        ..Default::default()
    })
}

fn exceeded(
    result: StoreResult<()>,
    what: &'static str,
    needed: u64,
    available: u64,
) -> TestResult {
    match result {
        Err(StoreError::BudgetExceeded {
            what: actual,
            needed: actual_needed,
            available: actual_available,
        }) => {
            check!(eq; actual, what);
            check!(eq; actual_needed, needed);
            check!(eq; actual_available, available);
            Ok(())
        }
        _ => Err("inventory preflight did not enforce the expected exact ceiling".into()),
    }
}

#[test]
fn measured_cohort_inventory_refuses_old_bytes_and_fits_production_budget() -> TestResult {
    let measured = inventory(1, 935_578_986)?;
    exceeded(
        preflight(
            measured,
            ReadBudget {
                bytes: 536_870_912,
                ..BUDGET
            },
        ),
        "capture encoded bytes",
        935_578_986,
        536_870_912,
    )?;
    preflight(measured, BUDGET)?;
    Ok(())
}

#[test]
fn production_inventory_bytes_are_inclusive_and_refuse_one_over() -> TestResult {
    check!(eq; BUDGET.bytes, 4_294_967_296);
    preflight(inventory(1, BUDGET.bytes)?, BUDGET)?;
    let excess = BUDGET.bytes.checked_add(1).ok_or("byte overflow")?;
    exceeded(
        preflight(inventory(1, excess)?, BUDGET),
        "capture encoded bytes",
        excess,
        BUDGET.bytes,
    )
}

#[test]
fn production_physical_rows_remain_inclusive_and_refuse_before_bytes() -> TestResult {
    check!(eq; BUDGET.rows, 2_097_152);
    check!(eq; BUDGET.record_bytes, 1_048_576);
    preflight(inventory(BUDGET.rows, 1)?, BUDGET)?;
    let rows = BUDGET.rows.checked_add(1).ok_or("row overflow")?;
    let bytes = BUDGET.bytes.checked_add(1).ok_or("byte overflow")?;
    exceeded(
        preflight(inventory(rows, bytes)?, BUDGET),
        "capture physical rows",
        rows,
        BUDGET.rows,
    )
}

#[test]
fn recorded_physical_counts_refuse_original_row_ceiling_and_fit_current_bound() -> TestResult {
    for rows in [1_796_172, 1_967_509, 1_220_513] {
        let counts = inventory(rows, 1)?;
        exceeded(
            preflight(
                counts,
                ReadBudget {
                    rows: 1_048_576,
                    ..BUDGET
                },
            ),
            "capture physical rows",
            rows,
            1_048_576,
        )?;
        preflight(counts, BUDGET)?;
    }
    Ok(())
}
