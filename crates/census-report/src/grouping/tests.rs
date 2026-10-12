use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exact_stage_bytes_and_work_are_admitted_but_one_more_is_refused() -> TestResult {
    let mut admission = Admission::new("fixture");
    admission.charge(STAGE_BYTES, STAGE_WORK)?;
    check!(eq; admission.charge(1, 0), Err(GroupingError::BudgetExceeded {
        stage: "fixture", group: "complete stage".to_owned(), resource: "encoded bytes",
        needed: STAGE_BYTES.checked_add(1).ok_or("overflow")?, available: STAGE_BYTES,
    }));
    check!(eq; admission.charge(0, 1), Err(GroupingError::BudgetExceeded {
        stage: "fixture", group: "complete stage".to_owned(), resource: "work units",
        needed: STAGE_WORK.checked_add(1).ok_or("overflow")?, available: STAGE_WORK,
    }));
    Ok(())
}

#[test]
fn arithmetic_overflow_is_not_a_saturated_count() -> TestResult {
    check!(eq; add("fixture", "claims", usize::MAX, 1), Err(GroupingError::Overflow {
        stage: "fixture", resource: "claims",
    }));
    Ok(())
}

#[test]
fn exact_encoded_row_budget_accepts_json_quotes_and_refuses_the_next_byte() -> TestResult {
    let string_bytes = ROW_BYTES.checked_sub(2).ok_or("budget too small")?;
    check!(eq; encoded_size("fixture", &"x".repeat(string_bytes))?, ROW_BYTES);
    check!(eq; encoded_size("fixture", &"x".repeat(ROW_BYTES)), Err(GroupingError::BudgetExceeded {
        stage: "fixture", group: "input record".to_owned(), resource: "encoded row bytes",
        needed: ROW_BYTES.checked_add(1).ok_or("overflow")?, available: ROW_BYTES,
    }));
    Ok(())
}
