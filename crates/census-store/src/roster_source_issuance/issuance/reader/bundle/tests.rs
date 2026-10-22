use super::*;
use std::io::Write;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn actual_replay_length_guard_admits_exact_and_refuses_one_extra_numerically() -> TestResult {
    admit_bundle_length(MAX_BUNDLE)?;
    check!(matches!(
        admit_bundle_length(MAX_BUNDLE.checked_add(1).ok_or("bundle size overflow")?),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    Ok(())
}

#[test]
fn actual_frozen_encoder_counter_admits_exact_then_refuses_one_more() -> TestResult {
    let mut count = Count(MAX_BUNDLE.checked_sub(1).ok_or("bundle size underflow")?);
    count.write_all(b"a")?;
    check!(eq; count.0, MAX_BUNDLE);
    check!(count.write(b"b").is_err());
    check!(eq; count.0, MAX_BUNDLE + 1);
    check!(count.write(b"").is_err());
    Ok(())
}

#[test]
fn actual_frozen_encoder_counter_refuses_arithmetic_overflow() -> TestResult {
    let mut count = Count(usize::MAX);
    check!(count.write(b"a").is_err());
    check!(eq; count.0, usize::MAX);
    Ok(())
}
