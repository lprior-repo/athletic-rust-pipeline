use crate::restate_services::plan::{plan, BrowserLaneState};
use crate::restate_services::wire::{
    HistoryWindow, JurisdictionRequest, JurisdictionState, SourcePlan,
};
use census_domain::model::Revision;
use census_domain::{model::SchoolYear, UsJurisdiction};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn athleticlive_missing_manifest_has_a_named_owed_obligation() -> TestResult {
    let request = JurisdictionRequest {
        jurisdiction: UsJurisdiction::Wisconsin,
        season: SchoolYear::new(2026).ok_or("season")?,
        revision: Revision(1),
        history: HistoryWindow::cohort("2026-10-12")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    let state = JurisdictionState {
        plan: Some(SourcePlan::of(
            &plan(request.jurisdiction, BrowserLaneState::Absent),
            "fixture".into(),
        )),
        ..JurisdictionState::default()
    };
    let (owed, _) = super::owed_work(&state, &request);
    check!(
        owed.iter().any(|stage| stage == "athleticlive_manifest"),
        "AthleticLIVE manifest obligation absent: {owed:?}"
    );
    Ok(())
}
