use super::*;

#[test]
fn independent_handles_for_one_budget_share_quota_and_owner() -> TestResult {
    let (budget, _) = bounded_budget(10)?;
    let mut first = budget.funding();
    let mut second = budget.funding();
    let left = first.reserve(storage(6)?)?;
    let right = second.reserve(storage(4)?)?;
    check!(left.same_owner(&right));
    check!(eq; first.ensure_owner(&right), Ok(()));
    check!(eq; second.ensure_owner(&left), Ok(()));
    check!(matches!(
        first.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(left);
    let reused = second.reserve(storage(6)?)?;
    check!(eq; reused.reserved_bytes().get(), 6);
    check!(matches!(
        first.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    Ok(())
}

#[test]
fn distinct_budgets_of_equal_size_never_share_owner_or_quota() -> TestResult {
    let (first_budget, _) = bounded_budget(8)?;
    let (second_budget, _) = bounded_budget(8)?;
    let mut first = first_budget.funding();
    let mut second = second_budget.funding();
    let left = first.reserve(storage(8)?)?;
    let right = second.reserve(storage(8)?)?;
    check!(!left.same_owner(&right));
    check!(eq; first.ensure_owner(&right), Err(HistoryFundingError::ForeignOwner));
    check!(eq; second.ensure_owner(&left), Err(HistoryFundingError::ForeignOwner));
    check!(eq; left.reserved_bytes().get(), 8);
    check!(eq; right.reserved_bytes().get(), 8);
    drop(left);
    check!(eq; first.reserve(storage(8)?)?.reserved_bytes().get(), 8);
    check!(matches!(
        second.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    Ok(())
}

struct AlternateHost(NonZeroU64);

impl super::super::HeldHistoryMemory for AlternateHost {
    fn reserved_bytes(&self) -> NonZeroU64 {
        self.0
    }
}

#[test]
fn different_parent_types_cannot_transfer_quota_between_ledgers() -> TestResult {
    let (first_budget, _) = bounded_budget(8)?;
    let second_budget = HistoryMemoryBudget::new(AlternateHost(
        NonZeroU64::new(8).ok_or("invalid test capacity")?,
    ));
    let mut first = first_budget.funding();
    let mut second = second_budget.funding();
    let mut left = first.reserve(storage(4)?)?;
    let right = second.reserve(storage(3)?)?;
    check!(eq; left.try_absorb(right), Err(HistoryFundingError::ForeignOwner));
    check!(eq; left.reserved_bytes().get(), 4);
    let returned = second.reserve(storage(8)?)?;
    check!(eq; returned.reserved_bytes().get(), 8);
    check!(matches!(
        first.reserve(storage(5)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(left);
    check!(eq; first.reserve(storage(8)?)?.reserved_bytes().get(), 8);
    Ok(())
}
