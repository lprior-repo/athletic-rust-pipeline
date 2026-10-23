use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_nces_scope_accepts_runtime_order_and_preserves_historical_order() -> TestResult {
    let runtime: Vec<_> = UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .filter(|state| FIVE_STATE_SCOPE.contains(state))
        .collect();
    check!(ne; runtime.as_slice(), FIVE_STATE_SCOPE.as_slice());
    NcesAddressRunReceiptV1::validate_scope(&runtime)?;
    NcesAddressRunReceiptV1::validate_scope(&FIVE_STATE_SCOPE)?;
    for shift in 0..FIVE_STATE_SCOPE.len() {
        let mut reordered = FIVE_STATE_SCOPE;
        reordered.rotate_left(shift);
        NcesAddressRunReceiptV1::validate_scope(&reordered)?;
    }
    Ok(())
}

#[test]
fn selected_nces_scope_refuses_duplicate_foreign_missing_and_extra_jurisdictions() -> TestResult {
    for state in [UsJurisdiction::California, UsJurisdiction::Michigan] {
        let invalid = [
            state,
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
        ];
        check!(eq; NcesAddressRunReceiptV1::validate_scope(&invalid),
            Err(NcesArtifactReceiptError::Scope));
    }
    let mut missing = FIVE_STATE_SCOPE.to_vec();
    missing.pop().ok_or("missing scope element")?;
    check!(NcesAddressRunReceiptV1::validate_scope(&missing).is_err());
    let mut extra = FIVE_STATE_SCOPE.to_vec();
    extra.push(UsJurisdiction::California);
    check!(NcesAddressRunReceiptV1::validate_scope(&extra).is_err());
    check!(NcesAddressRunReceiptV1::validate_scope(&[]).is_err());
    Ok(())
}
