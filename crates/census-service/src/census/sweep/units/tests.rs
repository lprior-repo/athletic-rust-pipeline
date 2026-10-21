use super::*;
use census_domain::UsJurisdiction;
use census_store::StoreError;

mod fixture;
use fixture::{team, TestResult};

#[test]
fn owned_pending_accepts_exact_four_mib_and_refuses_one_more_before_cloning() -> TestResult {
    let exact = fixture::inputs_at(4 * 1024 * 1024)?;
    check!(eq; owned_pending_rosters(&exact)?, exact);
    let mut over = exact;
    over.first_mut()
        .ok_or("bounded inputs missing")?
        .city_state
        .push('x');
    let error = owned_pending_rosters(&over)
        .err()
        .ok_or("oversized owned inputs accepted")?;
    check!(
        matches!(error, CrawlError::Store(StoreError::Refused { detail })
        if detail == "configured roster input bytes 4194305 exceed 4194304")
    );
    let (_root, store) = census_store::test_fixture::temp_store()?;
    check!(matches!(crate::census::scope::pending_rosters(
        &store, &over, UsJurisdiction::Michigan, SchoolYear::DEFAULT, fixture::revision()?,
    ), Err(StoreError::Refused { detail })
        if detail == "configured roster input bytes 4194305 exceed 4194304"));
    Ok(())
}

#[test]
fn owned_pending_keeps_original_twenty_thousand_input_ceiling() -> TestResult {
    let teams = vec![team("12439"); 20_001];
    check!(matches!(owned_pending_rosters(&teams),
        Err(CrawlError::Store(StoreError::TooManyRows { table, max:20_000 }))
            if table == "configured roster inputs"));
    Ok(())
}

#[test]
fn owned_pending_revisits_raw_done_rows_without_changing_prior_journal() -> TestResult {
    let (_root, store) = census_store::test_fixture::temp_store()?;
    let teams = vec![team("12439"), team("12440")];
    fixture::raw_done(&store, teams.first().ok_or("first team")?)?;
    let phase = crate::census::rosters_phase(
        UsJurisdiction::Michigan,
        SchoolYear::DEFAULT,
        fixture::revision()?,
    );
    let prior = store.journal_payload(&phase, "MI:12439")?;
    check!(eq; crate::census::scope::pending_rosters(
        &store, &teams, UsJurisdiction::Michigan, SchoolYear::DEFAULT, fixture::revision()?,
    )?, teams.get(1..).ok_or("remaining teams")?);
    let revision = (
        store.snapshot().evidence_generation(),
        store.snapshot().derived_generation(),
    );
    let sequence = store.snapshot().sequence();
    check!(eq; owned_pending_rosters(&teams)?, teams);
    check!(eq; store.journal_payload(&phase, "MI:12439")?, prior);
    check!(eq; (store.snapshot().evidence_generation(), store.snapshot().derived_generation()), revision);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}
