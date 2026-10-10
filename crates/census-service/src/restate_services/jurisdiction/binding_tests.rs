use census_domain::model::{CensusRun, GradYear};

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn season(year: i16) -> TestResult<SchoolYear> {
    Ok(SchoolYear::new(year).ok_or("fixture season")?)
}

fn bound(season_year: i16, revision: u32) -> TestResult<RunManifest> {
    Ok(RunManifest {
        store_identity: "store-fixture".to_string(),
        run: CensusRun::new(season(season_year)?, revision).ok_or("fixture run")?,
        cohort: GradYear::CO2027,
        jurisdictions: Vec::new(),
    })
}

#[test]
fn an_unbound_store_admits_any_revision() -> TestResult {
    check!(bound_run_refusal(None, season(2026)?, Revision(7)).is_none());
    Ok(())
}

#[test]
fn the_bound_run_admits_its_own_revision() -> TestResult {
    let manifest = bound(2026, 1)?;
    check!(bound_run_refusal(Some(&manifest), season(2026)?, Revision(1)).is_none());
    Ok(())
}

#[test]
fn a_bound_store_refuses_another_revision_and_names_both() -> TestResult {
    let manifest = bound(2026, 1)?;
    let refusal = bound_run_refusal(Some(&manifest), season(2026)?, Revision(2))
        .ok_or("a stray revision is refused")?;
    check!(refusal.contains("run 2026-1"), "{refusal}");
    check!(refusal.contains("2026-2"), "{refusal}");
    Ok(())
}

#[test]
fn a_bound_store_refuses_another_season() -> TestResult {
    let manifest = bound(2026, 1)?;
    check!(bound_run_refusal(Some(&manifest), season(2027)?, Revision(1)).is_some());
    Ok(())
}
