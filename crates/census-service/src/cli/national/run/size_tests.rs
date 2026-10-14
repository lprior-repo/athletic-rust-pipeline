use super::next_resume_ordinal;
use std::num::NonZeroU32;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn national_drive_first_retry_uses_resume_two() -> TestResult {
    check!(eq; next_resume_ordinal(None)?.get(), 2);
    Ok(())
}

#[test]
fn national_drive_retry_advances_selected_ordinal_once() -> TestResult {
    let seven = NonZeroU32::new(7).ok_or("fixture ordinal")?;
    check!(eq; next_resume_ordinal(Some(seven))?.get(), 8);
    let penultimate = NonZeroU32::new(u32::MAX - 1).ok_or("fixture ordinal")?;
    check!(eq; next_resume_ordinal(Some(penultimate))?.get(), u32::MAX);
    Ok(())
}

#[test]
fn national_drive_retry_refuses_ordinal_exhaustion_exactly() -> TestResult {
    let final_ordinal = NonZeroU32::new(u32::MAX).ok_or("fixture ordinal")?;
    match next_resume_ordinal(Some(final_ordinal)) {
        Err(error) => check!(eq; error.to_string(), "resume ordinal exhausted"),
        Ok(ordinal) => return Err(format!("exhausted ordinal advanced to {ordinal}").into()),
    }
    Ok(())
}
