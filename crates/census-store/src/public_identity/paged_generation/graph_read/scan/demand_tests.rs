use super::preflight;
use crate::public_identity::authority::graph::admission::demand::{
    entry_bytes, require_raw_inventory, RAW_BUDGET,
};
use crate::public_identity::authority::graph::BUDGET;
use crate::public_identity::paged_generation::{wire, InventoryV2};
use crate::public_identity_tests::support::TestResult;
use crate::{StoreError, StoreResult};
use serde_json::json;

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
        _ => Err("raw inventory did not return the exact bounded refusal".into()),
    }
}

#[test]
fn diagnostic_and_normal_budgets_keep_independent_byte_ceilings() -> TestResult {
    check!(eq; BUDGET.bytes, 17_179_869_184);
    check!(eq; BUDGET.rows, 2_097_152);
    check!(eq; BUDGET.record_bytes, 1_048_576);
    check!(eq; RAW_BUDGET.bytes, 34_359_738_368);
    check!(eq; RAW_BUDGET.rows, 2_097_152);
    check!(eq; RAW_BUDGET.record_bytes, 1_048_576);
    Ok(())
}

#[test]
fn measured_980_backing_demand_fits_finite_normal_and_diagnostic_capacity() -> TestResult {
    let measured = inventory(1_796_172, 7_440_378_517)?;
    exceeded(
        preflight(
            measured,
            crate::ReadBudget {
                bytes: 4_294_967_296,
                ..BUDGET
            },
        ),
        "capture encoded bytes",
        7_440_378_517,
        4_294_967_296,
    )?;
    preflight(measured, BUDGET)?;
    require_raw_inventory(measured)?;
    Ok(())
}

#[test]
fn diagnostic_raw_bytes_accept_exact_32_gib_and_refuse_one_more_byte() -> TestResult {
    require_raw_inventory(inventory(1, 34_359_738_368)?)?;
    exceeded(
        require_raw_inventory(inventory(1, 34_359_738_369)?),
        "capture encoded bytes",
        34_359_738_369,
        34_359_738_368,
    )
}

#[test]
fn diagnostic_physical_rows_accept_exact_bound_and_refuse_one_more_row() -> TestResult {
    require_raw_inventory(inventory(2_097_152, 1)?)?;
    exceeded(
        require_raw_inventory(inventory(2_097_153, 1)?),
        "capture physical rows",
        2_097_153,
        2_097_152,
    )
}

#[test]
fn diagnostic_preserves_physical_row_refusal_before_encoded_byte_refusal() -> TestResult {
    exceeded(
        require_raw_inventory(inventory(2_097_153, 34_359_738_369)?),
        "capture physical rows",
        2_097_153,
        2_097_152,
    )
}

#[test]
fn shared_entry_size_matches_independent_literal_envelope_and_newline() -> TestResult {
    let row = json!({
        "name": "Quoted \"Athlete\"",
        "note": "first\nsecond\\last",
        "school": "École",
        "year": 2027
    });
    let literal = json!({
        "key": "athlete:00000000000000000000",
        "at": "",
        "payload": {"kind": "athlete", "value": &row}
    });
    let mut expected = serde_json::to_vec(&literal)?;
    expected.push(b'\n');
    check!(eq; entry_bytes("athlete", 0, &row)?, expected.len());
    Ok(())
}

#[test]
fn shared_entry_size_preserves_full_u64_ordinal_and_observation_kind() -> TestResult {
    let row = json!({"source": "milesplit_athlete", "id": "14789618"});
    let literal = json!({
        "key": "observation:18446744073709551615",
        "at": "",
        "payload": {"kind": "observation", "value": &row}
    });
    let mut expected = serde_json::to_vec(&literal)?;
    expected.push(b'\n');
    check!(eq; entry_bytes("observation", u64::MAX, &row)?, expected.len());
    Ok(())
}

fn full_record_value() -> TestResult<String> {
    let empty = json!({
        "key": "athlete:00000000000000000000",
        "at": "",
        "payload": {"kind": "athlete", "value": ""}
    });
    let overhead = serde_json::to_vec(&empty)?
        .len()
        .checked_add(1)
        .ok_or("entry envelope overflow")?;
    let padding = 1_048_576_usize
        .checked_sub(overhead)
        .ok_or("entry envelope exceeds record limit")?;
    Ok("x".repeat(padding))
}

#[test]
fn shared_entry_record_boundary_includes_envelope_and_newline() -> TestResult {
    let value = full_record_value()?;
    let literal = json!({
        "key": "athlete:00000000000000000000",
        "at": "",
        "payload": {"kind": "athlete", "value": &value}
    });
    let mut expected = serde_json::to_vec(&literal)?;
    expected.push(b'\n');
    check!(eq; expected.len(), 1_048_576);
    check!(eq; entry_bytes("athlete", 0, &value)?, 1_048_576);
    let excess = format!("{value}x");
    match entry_bytes("athlete", 0, &excess) {
        Err(StoreError::Json { detail, source }) => {
            check!(eq; detail, "encoding bounded public identity state");
            check!(source.is_io());
        }
        _ => return Err("one-byte record excess did not retain the bounded encoder error".into()),
    }
    Ok(())
}
