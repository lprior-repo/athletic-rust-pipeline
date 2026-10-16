use super::*;

#[test]
fn split_partitions_a_grant_without_extra_admission_or_counter_change() -> TestResult {
    let (budget, _) = bounded_budget(8)?;
    let mut funding = budget.funding();
    let mut receiver = funding.reserve(storage(8)?)?;
    let portion = receiver.try_split(smaller(3)?)?;
    check!(eq; receiver.reserved_bytes().get(), 5);
    check!(eq; portion.reserved_bytes().get(), 3);
    check!(receiver.same_owner(&portion));
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(portion);
    check!(matches!(
        funding.reserve(storage(4)?),
        Err(HistoryFundingError::NoBytes)
    ));
    let refill = funding.reserve(storage(3)?)?;
    check!(eq; receiver.reserved_bytes().get(), 5);
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(receiver);
    drop(refill);
    check!(eq; funding.reserve(storage(8)?)?.reserved_bytes().get(), 8);
    Ok(())
}

#[test]
fn equal_and_greater_split_requests_are_refused_unchanged() -> TestResult {
    let (budget, _) = bounded_budget(5)?;
    let mut funding = budget.funding();
    let mut grant = funding.reserve(storage(5)?)?;
    check!(matches!(
        grant.try_split(smaller(5)?),
        Err(HistoryFundingError::InvalidSplit)
    ));
    check!(matches!(
        grant.try_split(smaller(6)?),
        Err(HistoryFundingError::InvalidSplit)
    ));
    check!(eq; grant.reserved_bytes().get(), 5);
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    let portion = grant.try_split(smaller(1)?)?;
    check!(eq; grant.reserved_bytes().get(), 4);
    check!(eq; portion.reserved_bytes().get(), 1);
    Ok(())
}
