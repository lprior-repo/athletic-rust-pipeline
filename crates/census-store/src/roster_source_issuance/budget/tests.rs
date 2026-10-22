use super::*;
use census_domain::roster_affiliation_authority::MAX_ROSTER_PROTECTED_BYTES;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn configured_byte_budget_is_exact_and_overage_permanently_refuses() -> TestResult {
    let mut budget = RosterStartupValidationBudgetV1::new();
    budget.charge(usize::try_from(MAX_ROSTER_PROTECTED_BYTES)?)?;
    check!(eq; budget.bytes, 0);
    check!(matches!(
        budget.charge(1),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    check!(matches!(
        budget.charge(0),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    Ok(())
}

#[test]
fn final_row_is_admitted_then_row_overage_permanently_refuses() -> TestResult {
    let mut budget = RosterStartupValidationBudgetV1::new();
    budget.rows = 1;
    budget.charge(1)?;
    check!(eq; budget.rows, 0);
    check!(matches!(
        budget.charge(0),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    check!(eq; budget.bytes, 0);
    check!(matches!(
        budget.charge(0),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    Ok(())
}

#[test]
fn default_row_budget_refuses_only_after_exact_configured_count() -> TestResult {
    let mut budget = RosterStartupValidationBudgetV1::new();
    for _ in 0..census_domain::roster_affiliation_authority::MAX_ROSTER_PROTECTED_ROWS {
        budget.charge(0)?;
    }
    check!(matches!(
        budget.charge(0),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    check!(matches!(
        budget.charge(0),
        Err(crate::StoreError::RosterSourceIssuance(
            Error::BudgetExceeded
        ))
    ));
    Ok(())
}
