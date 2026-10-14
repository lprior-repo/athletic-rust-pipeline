use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn write_call_and_successful_byte_envelopes_refuse_the_first_excess() -> TestResult {
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    admission.write_calls.store(
        SPILL_WRITE_CALLS.checked_sub(1).ok_or("calls")?,
        Ordering::Release,
    );
    admission.before_write()?;
    check!(
        matches!(admission.before_write(), Err(StoreError::BudgetExceeded {
        what: "results spill write calls", needed, available,
    }) if needed == SPILL_WRITE_CALLS + 1 && available == SPILL_WRITE_CALLS)
    );
    admission.written_bytes.store(
        CAPTURE_BYTES.checked_sub(1).ok_or("bytes")?,
        Ordering::Release,
    );
    admission.after_write(1)?;
    check!(matches!(
        admission.after_write(1),
        Err(StoreError::BudgetExceeded {
            what: "results spill written bytes",
            needed: 13_958_643_713,
            available: 13_958_643_712,
        })
    ));
    drop(lease);
    drop(admission.claim()?);
    check!(eq; admission.writes(), (0, 0));
    Ok(())
}
