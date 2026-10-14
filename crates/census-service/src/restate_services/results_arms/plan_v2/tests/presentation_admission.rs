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

#[test]
fn schema_quarantine_roundtrips_with_exact_execution_allocation() -> TestResult {
    let plan = schema_quarantine_plan(1)?;
    let encoded = encode(&plan)?;
    let mut exact = decode(&mut encoded.clone())?;
    check!(eq; encode(&exact)?, encoded);
    let Discovery::Ready(ready) = exact.discovery.first().ok_or("fixture discovery")? else {
        return Err("schema quarantine did not replay as Ready".into());
    };
    check!(eq; ready.pages_read, 0);
    check!(ready.options.urls.is_empty());
    check!(eq; ready.quarantines.len(), 1);
    let quarantine = ready.quarantines.first().ok_or("fixture quarantine")?;
    check!(eq; quarantine.url, exact.meets.first().ok_or("fixture meet")?.results_url);
    check!(eq; quarantine.reason.as_str(), "schema fixture: not a results page");
    let length = diagnostic::count(
        &format_args!(
            "quarantined meet page {}: {}",
            quarantine.url, quarantine.reason
        ),
        8192,
    )?;
    let required = length
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(quarantine.url.len()))
        .and_then(|bytes| bytes.checked_add(128))
        .and_then(|bytes| bytes.checked_add(65_536))
        .and_then(|bytes| {
            bytes.checked_add(std::mem::size_of::<
                crate::restate_services::results_arms::ResultsSourceRows,
            >())
        })
        .ok_or("fixture quarantine presentation bytes")?;
    let required = u64::try_from(required)?;
    let remaining = exact.allocation.remaining()?;
    exact.allocation.charge_u64(
        remaining
            .checked_sub(required)
            .ok_or("fixture exact allowance")?,
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
fn quarantine_wire_boundary_preserves_twenty_five_and_refuses_twenty_six() -> TestResult {
    let plan = schema_quarantine_plan(25)?;
    let mut encoded = encode(&plan)?;
    let replay = decode(&mut encoded)?;
    let Discovery::Ready(ready) = replay.discovery.first().ok_or("fixture discovery")? else {
        return Err("boundary quarantine did not replay as Ready".into());
    };
    check!(eq; ready.quarantines.len(), 25);
    let mut hostile = serde_json::to_value(&plan)?;
    let quarantines = hostile
        .get_mut("discovery")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|values| values.first_mut())
        .and_then(|discovery| discovery.get_mut("ready"))
        .and_then(|ready| ready.get_mut("quarantines"))
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("fixture quarantine array")?;
    let extra = quarantines.first().cloned().ok_or("fixture quarantine")?;
    quarantines.push(extra);
    let mut bytes = Bytes::from(serde_json::to_vec(&hostile)?);
    json_error(decode(&mut bytes), "sequence limit")?;
    check!(matches!(
        encode(&schema_quarantine_plan(26)?),
        Err(ResultsPlanError::Invalid("discovery cardinalities"))
    ));
    Ok(())
}

fn schema_quarantine_plan(count: usize) -> TestResult<ResultsPlanV2> {
    let fixture = rebinding::Fixture::new()?;
    let mut plan = fixture.plan()?;
    let url = plan
        .meets
        .first()
        .ok_or("fixture meet")?
        .results_url
        .clone();
    let quarantines = (0..count)
        .map(|_| wire::Quarantine {
            url: url.clone(),
            reason: "schema fixture: not a results page".to_owned(),
        })
        .collect();
    *plan.discovery.first_mut().ok_or("fixture discovery")? = Discovery::Ready(ReadyDiscovery {
        options: census_crawl::milesplit::ResultSetOptions::default(),
        pages_read: 0,
        quarantines,
    });
    Ok(plan)
}
