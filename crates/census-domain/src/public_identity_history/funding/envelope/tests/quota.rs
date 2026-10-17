use super::*;

#[test]
fn exact_capacity_is_admitted_and_one_byte_beyond_is_refused() -> TestResult {
    let (budget, _) = bounded_budget(9)?;
    let mut funding = budget.funding();
    let full = funding.reserve(storage(9)?)?;
    check!(eq; full.reserved_bytes().get(), 9);
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(full);
    check!(eq; funding.reserve(storage(9)?)?.reserved_bytes().get(), 9);
    Ok(())
}

#[test]
fn one_byte_short_refusal_preserves_the_prior_live_grant() -> TestResult {
    let (budget, _) = bounded_budget(12)?;
    let mut first = budget.funding();
    let mut second = budget.funding();
    let live = first.reserve(storage(8)?)?;
    check!(matches!(
        second.reserve(storage(5)?),
        Err(HistoryFundingError::NoBytes)
    ));
    check!(eq; live.reserved_bytes().get(), 8);
    let exact = second.reserve(storage(4)?)?;
    check!(eq; exact.reserved_bytes().get(), 4);
    check!(live.same_owner(&exact));
    drop(live);
    check!(eq; second.reserve(storage(8)?)?.reserved_bytes().get(), 8);
    Ok(())
}

#[test]
fn dropped_grant_re_admits_its_capacity_exactly_once() -> TestResult {
    let (budget, _) = bounded_budget(16)?;
    let mut funding = budget.funding();
    let grant = funding.reserve(storage(16)?)?;
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(grant);
    let again = funding.reserve(storage(16)?)?;
    check!(matches!(
        funding.reserve(storage(1)?),
        Err(HistoryFundingError::NoBytes)
    ));
    drop(again);
    check!(eq; funding.reserve(storage(16)?)?.reserved_bytes().get(), 16);
    Ok(())
}

#[test]
fn storage_refusal_keeps_the_existing_funded_value() -> TestResult {
    let (budget, _) = bounded_budget(8)?;
    let mut funding = budget.funding();
    let mut bytes = OwnedHistoryBytes::with_capacity(8, &mut funding)?;
    bytes.extend_from_slice(b"kept")?;
    check!(matches!(
        OwnedHistoryBytes::with_capacity(1, &mut funding),
        Err(HistoryStorageError::Funding(HistoryFundingError::NoBytes))
    ));
    check!(eq; bytes.as_bytes(), b"kept");
    drop(bytes);
    check!(eq; funding.reserve(storage(8)?)?.reserved_bytes().get(), 8);
    Ok(())
}
