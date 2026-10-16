use super::*;

#[test]
fn successful_absorb_holds_the_combined_charge_until_the_receiver_drops() -> TestResult {
    let (budget, _) = bounded_budget(10)?;
    let mut left = budget.funding();
    let mut right = budget.funding();
    let mut receiver = left.reserve(storage(4)?)?;
    let donor = right.reserve(storage(6)?)?;
    check!(receiver.same_owner(&donor));
    check!(eq; receiver.try_absorb(donor), Ok(()));
    check!(eq; receiver.reserved_bytes().get(), 10);
    check!(matches!(
        left.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(receiver);
    let readmitted = left.reserve(storage(10)?)?;
    check!(eq; readmitted.reserved_bytes().get(), 10);
    check!(matches!(
        left.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    Ok(())
}

#[test]
fn foreign_absorb_keeps_the_receiver_and_refunds_the_donor() -> TestResult {
    let (first_budget, _) = bounded_budget(5)?;
    let (second_budget, _) = bounded_budget(6)?;
    let mut first = first_budget.funding();
    let mut second = second_budget.funding();
    let mut receiver = first.reserve(storage(5)?)?;
    let donor = second.reserve(storage(6)?)?;
    check!(!receiver.same_owner(&donor));
    check!(eq; receiver.try_absorb(donor), Err(HistoryFundingError::ForeignOwner));
    check!(eq; receiver.reserved_bytes().get(), 5);
    check!(matches!(
        first.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    check!(eq; second.reserve(storage(6)?)?.reserved_bytes().get(), 6);
    Ok(())
}
