use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn all_49_jurisdictions_fit_the_aggregate_row_boundary_and_release() -> TestResult {
    let admission = Arc::new(Admission::default());
    let mut permits = Vec::new();
    for _ in 0..49 {
        let mut permit = Permit::new(Arc::clone(&admission));
        permit.reserve(SELECTION_ROWS, 0)?;
        permits.push(permit);
    }
    check!(eq; admission.rows.load(Ordering::Acquire), GLOBAL_ROWS);
    let mut extra = Permit::new(Arc::clone(&admission));
    check!(matches!(
        extra.reserve(1, 0),
        Err(StoreError::BudgetExceeded {
            what: "results global selected rows",
            needed: 3_211_265,
            available: 3_211_264
        })
    ));
    drop(permits);
    check!(eq; admission.rows.load(Ordering::Acquire), 0);
    extra.reserve(SELECTION_ROWS, 0)?;
    drop(extra);
    check!(eq; admission.rows.load(Ordering::Acquire), 0);
    Ok(())
}

#[test]
fn aggregate_bytes_refuse_the_first_extra_byte_without_leaking_row_admission() -> TestResult {
    let admission = Arc::new(Admission::default());
    let mut full = Permit::new(Arc::clone(&admission));
    full.reserve(1, GLOBAL_BYTES)?;
    let mut extra = Permit::new(Arc::clone(&admission));
    check!(matches!(
        extra.reserve(1, 1),
        Err(StoreError::BudgetExceeded {
            what: "results global selected bytes",
            needed: 1_073_741_825,
            available: 1_073_741_824
        })
    ));
    check!(eq; admission.rows.load(Ordering::Acquire), 1);
    check!(eq; admission.bytes.load(Ordering::Acquire), GLOBAL_BYTES);
    drop(full);
    extra.reserve(1, GLOBAL_BYTES)?;
    drop(extra);
    check!(eq; admission.rows.load(Ordering::Acquire), 0);
    check!(eq; admission.bytes.load(Ordering::Acquire), 0);
    Ok(())
}

#[test]
fn per_selection_rows_and_spill_disk_bytes_are_checked_at_exact_boundaries() -> TestResult {
    let admission = Arc::new(Admission::default());
    let mut permit = Permit::new(Arc::clone(&admission));
    permit.reserve(SELECTION_ROWS, 0)?;
    check!(matches!(
        permit.reserve(1, 0),
        Err(StoreError::BudgetExceeded {
            what: "results selected rows",
            needed: 65_537,
            available: 65_536
        })
    ));
    let spill = Arc::new(SpillAdmission::default());
    let lease = spill.claim()?;
    spill.charge(CAPTURE_BYTES)?;
    check!(matches!(
        spill.charge(1),
        Err(StoreError::BudgetExceeded {
            what: "results spill bytes",
            needed: 13_958_643_713,
            available: 13_958_643_712,
        })
    ));
    drop(lease);
    check!(eq; spill.reserved_bytes()?, 0);
    check!(matches!(
        charge(u64::MAX, 1, u64::MAX, "overflow"),
        Err(StoreError::CounterOverflow)
    ));
    drop(permit);
    check!(eq; admission.rows.load(Ordering::Acquire), 0);
    Ok(())
}
