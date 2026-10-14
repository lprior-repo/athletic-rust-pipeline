use super::*;

#[test]
fn decoded_ordinary_failure_presentation_requires_its_cumulative_allowance() -> TestResult {
    let fixture = rebinding::Fixture::new()?;
    let encoded = encode(&fixture.plan()?)?;
    let mut exact = decode(&mut encoded.clone())?;
    let required = 65_536_usize
        .checked_add(std::mem::size_of::<
            crate::restate_services::results_arms::ResultsSourceRows,
        >())
        .ok_or("fixture presentation bytes")?;
    let required = u64::try_from(required)?;
    exact.allocation.charge_u64(
        exact
            .allocation
            .remaining()?
            .checked_sub(required)
            .ok_or("fixture available allocation")?,
    )?;
    exact.reserve_execution()?;
    check!(eq; exact.allocation.remaining()?, 0);
    let mut short = decode(&mut encoded.clone())?;
    let spend = short
        .allocation
        .remaining()?
        .checked_sub(required)
        .and_then(|bytes| bytes.checked_add(1))
        .ok_or("fixture one-short allowance")?;
    short.allocation.charge_u64(spend)?;
    check!(matches!(
        short.reserve_execution(),
        Err(ResultsPlanError::Limit("allocation"))
    ));
    Ok(())
}

#[test]
fn decoded_ordinary_ready_candidate_reserves_failure_presentation_before_admission() -> TestResult {
    let fixture = rebinding::Fixture::new()?;
    let mut plan = fixture.plan()?;
    *plan.discovery.first_mut().ok_or("fixture discovery")? = Discovery::Ready(ReadyDiscovery {
        options: census_crawl::milesplit::ResultSetOptions::default(),
        pages_read: 0,
        quarantines: Vec::new(),
    });
    let mut plan = decode(&mut encode(&plan)?)?;
    let before = plan.allocation.remaining()?;
    plan.reserve_execution()?;
    let expected = 65_536_usize
        .checked_add(std::mem::size_of::<
            crate::restate_services::results_arms::ResultsSourceRows,
        >())
        .ok_or("fixture presentation bytes")?;
    check!(eq; before.checked_sub(plan.allocation.remaining()?), Some(u64::try_from(expected)?));
    Ok(())
}
