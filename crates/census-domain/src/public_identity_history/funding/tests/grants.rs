use super::*;

fn storage(bytes: usize) -> TestResult<ConstructionStorage> {
    ConstructionStorage::from_bytes(bytes).ok_or_else(|| "invalid test storage size".into())
}

#[test]
fn absorption_transfers_charge_without_a_donor_refund() -> TestResult {
    let mut funding = TestFunding::new(32);
    let mut first = funding.reserve(storage(3)?)?;
    let second = funding.reserve(storage(5)?)?;
    check!(first.same_owner(&second));
    first.try_absorb(second)?;
    check!(eq; first.reserved_bytes().get(), 8);
    check!(eq; funding.pool().available(), 24);
    check!(eq; funding.pool().live(), 8);
    check!(eq; funding.pool().peak(), 8);
    drop(first);
    check!(eq; funding.pool().available(), 32);
    check!(eq; funding.pool().live(), 0);
    check!(funding.pool().valid());
    Ok(())
}

#[test]
fn independent_pools_cannot_merge_even_with_identical_capacity() -> TestResult {
    let mut first_pool = TestFunding::new(32);
    let mut second_pool = TestFunding::new(32);
    let mut first = first_pool.reserve(storage(4)?)?;
    let second = second_pool.reserve(storage(6)?)?;
    check!(!first.same_owner(&second));
    check!(eq; first_pool.ensure_owner(&second), Err(Fault::Foreign));
    check!(eq; first.try_absorb(second), Err(Fault::Foreign));
    check!(eq; first.reserved_bytes().get(), 4);
    check!(eq; first_pool.pool().live(), 4);
    check!(eq; second_pool.pool().live(), 0);
    check!(eq; second_pool.pool().available(), 32);
    drop(first);
    check!(eq; first_pool.pool().available(), 32);
    check!(first_pool.pool().valid() && second_pool.pool().valid());
    Ok(())
}

#[test]
fn wrong_sized_reservation_is_refunded_and_never_retained() -> TestResult {
    let mut funding = TestFunding::oversized(16);
    check!(matches!(
        OwnedHistoryBytes::with_capacity(4, &mut funding),
        Err(HistoryStorageError::InvalidGrant)
    ));
    check!(eq; funding.pool().peak(), 5);
    check!(eq; funding.pool().live(), 0);
    check!(eq; funding.pool().available(), 16);
    check!(funding.pool().valid());
    Ok(())
}

#[test]
fn foreign_reservation_returns_to_the_actual_originating_pool() -> TestResult {
    let mut funding = TestFunding::foreign(16);
    check!(matches!(
        OwnedHistorySlots::<u32, _>::with_capacity(2, &mut funding),
        Err(HistoryStorageError::Funding(Fault::Foreign))
    ));
    check!(eq; funding.pool().peak(), 0);
    check!(eq; funding.supplied_pool().peak(), 8);
    check!(eq; funding.pool().available(), 16);
    check!(eq; funding.supplied_pool().available(), 16);
    check!(funding.pool().valid() && funding.supplied_pool().valid());
    Ok(())
}

#[test]
fn split_releases_only_the_transferred_portion() -> TestResult {
    let mut funding = TestFunding::new(8);
    let mut original = funding.reserve(storage(8)?)?;
    let three = NonZeroU64::new(3).ok_or("nonzero split")?;
    let portion = original.try_split(three)?;
    check!(eq; original.reserved_bytes().get(), 5);
    check!(eq; portion.reserved_bytes().get(), 3);
    check!(eq; funding.pool().live(), 8);
    check!(eq; funding.pool().available(), 0);
    drop(portion);
    check!(eq; funding.pool().live(), 5);
    check!(eq; funding.pool().available(), 3);
    check!(matches!(funding.reserve(storage(4)?), Err(Fault::Limit)));
    let reused = funding.reserve(storage(3)?)?;
    check!(eq; funding.pool().live(), 8);
    drop(reused);
    drop(original);
    check!(eq; funding.pool().available(), 8);
    check!(funding.pool().valid());
    Ok(())
}

#[test]
fn equal_or_larger_split_leaves_the_receiver_and_pool_unchanged() -> TestResult {
    let mut funding = TestFunding::new(8);
    let mut original = funding.reserve(storage(5)?)?;
    for amount in [5, 6] {
        let amount = NonZeroU64::new(amount).ok_or("nonzero split")?;
        check!(matches!(
            original.try_split(amount),
            Err(Fault::InvalidSplit)
        ));
        check!(eq; original.reserved_bytes().get(), 5);
        check!(eq; funding.pool().available(), 3);
        check!(eq; funding.pool().live(), 5);
    }
    drop(original);
    check!(eq; funding.pool().available(), 8);
    check!(funding.pool().valid());
    Ok(())
}
