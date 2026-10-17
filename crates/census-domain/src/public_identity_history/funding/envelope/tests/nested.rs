use super::*;

#[test]
fn nested_bytes_keep_the_original_host_grant_until_the_region_is_dropped() -> TestResult {
    let (parent, observed_drops) = bounded_budget(16)?;
    let mut parent_funding = parent.funding();
    let region = HistoryMemoryBudget::new(parent_funding.reserve(storage(12)?)?);
    let mut funding = region.funding();
    let mut bytes = OwnedHistoryBytes::with_capacity(12, &mut funding)?;
    bytes.extend_from_slice(b"witness-root")?;
    check!(eq; bytes.as_bytes(), b"witness-root");
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    check!(matches!(
        parent_funding.reserve(storage(5)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(bytes);
    let recycled = funding.reserve(storage(12)?)?;
    check!(matches!(
        parent_funding.reserve(storage(5)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(recycled);
    drop(funding);
    check!(eq; observed_drops.get(), 0);
    drop(region);
    let restored = parent_funding.reserve(storage(16)?)?;
    check!(eq; HistoryGrant::reserved_bytes(&restored).get(), 16);
    drop(restored);
    drop(parent_funding);
    drop(parent);
    check!(eq; observed_drops.get(), 1);
    Ok(())
}

#[test]
fn sibling_regions_refuse_each_others_grants_despite_a_shared_host_parent() -> TestResult {
    let (parent, observed_drops) = bounded_budget(16)?;
    let mut parent_funding = parent.funding();
    let left = HistoryMemoryBudget::new(parent_funding.reserve(storage(8)?)?);
    let right = HistoryMemoryBudget::new(parent_funding.reserve(storage(8)?)?);
    let mut left_funding = left.funding();
    let mut right_funding = right.funding();
    let left_grant = left_funding.reserve(storage(8)?)?;
    let mut right_grant = right_funding.reserve(storage(8)?)?;
    check!(matches!(
        right_funding.ensure_owner(&left_grant),
        Err(HistoryFundingError::ForeignOwner)
    ));
    check!(matches!(
        right_grant.try_absorb(left_grant),
        Err(HistoryFundingError::ForeignOwner)
    ));
    check!(eq; HistoryGrant::reserved_bytes(&right_grant).get(), 8);
    let left_reclaimed = left_funding.reserve(storage(8)?)?;
    check!(matches!(
        parent_funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(left_reclaimed);
    drop(right_grant);
    drop(left_funding);
    drop(right_funding);
    drop(left);
    drop(right);
    let restored = parent_funding.reserve(storage(16)?)?;
    check!(eq; HistoryGrant::reserved_bytes(&restored).get(), 16);
    drop(restored);
    drop(parent_funding);
    drop(parent);
    check!(eq; observed_drops.get(), 1);
    Ok(())
}
