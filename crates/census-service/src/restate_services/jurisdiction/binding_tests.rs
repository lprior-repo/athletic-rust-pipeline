use census_domain::model::{CensusRun, GradYear};

use super::*;

fn season(year: i16) -> SchoolYear {
    SchoolYear::new(year).expect("fixture season")
}

fn bound(season_year: i16, revision: u32) -> RunManifest {
    RunManifest {
        store_identity: "store-fixture".to_string(),
        run: CensusRun::new(season(season_year), revision).expect("fixture run"),
        cohort: GradYear::CO2027,
        jurisdictions: Vec::new(),
    }
}

#[test]
fn an_unbound_store_admits_any_revision() {
    assert!(bound_run_refusal(None, season(2026), Revision(7)).is_none());
}

#[test]
fn the_bound_run_admits_its_own_revision() {
    let manifest = bound(2026, 1);
    assert!(bound_run_refusal(Some(&manifest), season(2026), Revision(1)).is_none());
}

#[test]
fn a_bound_store_refuses_another_revision_and_names_both() {
    let manifest = bound(2026, 1);
    let refusal = bound_run_refusal(Some(&manifest), season(2026), Revision(2))
        .expect("a stray revision is refused");
    assert!(refusal.contains("run 2026-1"), "{refusal}");
    assert!(refusal.contains("2026-2"), "{refusal}");
}

#[test]
fn a_bound_store_refuses_another_season() {
    let manifest = bound(2026, 1);
    assert!(bound_run_refusal(Some(&manifest), season(2027), Revision(1)).is_some());
}
