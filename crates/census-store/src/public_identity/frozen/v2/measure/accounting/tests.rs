use super::*;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn array_lengths_match_independent_json_with_escaping_and_empty_items() -> TestResult {
    for values in [
        vec![],
        vec![""],
        vec!["a\"\\\n", "é\t"],
        vec!["first", "", "third"],
    ] {
        let mut array = JsonArrayBytes::default();
        for value in &values {
            array.push(serde_json::to_vec(value)?.len())?;
        }
        check!(eq; array.items(), values.len());
        check!(eq; checked_sum(array.content(), 2)?, serde_json::to_vec(&values)?.len());
    }
    Ok(())
}

#[test]
fn inclusive_budget_refuses_excess_and_overflow_without_mutation() -> TestResult {
    let mut budget = ByteBudget::new(5);
    budget.add(2)?;
    budget.add(3)?;
    check!(eq; budget.remaining()?, 0);
    check!(matches!(
        budget.add(1),
        Err(StoreError::BudgetExceeded {
            needed: 6,
            available: 5,
            ..
        })
    ));
    check!(eq; budget.used(), 5);
    let mut overflow = ByteBudget::new(usize::MAX);
    overflow.add(usize::MAX)?;
    check!(matches!(overflow.add(1), Err(StoreError::CounterOverflow)));
    check!(eq; overflow.used(), usize::MAX);
    let mut array = JsonArrayBytes::default();
    array.push(usize::MAX)?;
    check!(matches!(array.push(0), Err(StoreError::CounterOverflow)));
    check!(eq; array.items(), 1);
    check!(eq; array.content(), usize::MAX);
    Ok(())
}

#[test]
fn diagnostic_byte_counter_can_exceed_unchanged_normal_cap_without_a_bundle() -> TestResult {
    let normal = crate::MAX_FROZEN_PUBLIC_IDENTITY_BYTES;
    check!(eq; normal, 4_294_967_296);
    let mut diagnostic = ByteBudget::new(crate::MAX_FROZEN_PUBLIC_IDENTITY_DEMAND_JSON_BYTES);
    diagnostic.add(normal)?;
    diagnostic.add(1)?;
    check!(eq; diagnostic.used(), checked_sum(normal, 1)?);
    Ok(())
}
