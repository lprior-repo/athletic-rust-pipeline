use super::*;
use crate::restate_services::jurisdiction::team_source::tests::{bound_national_request, request};
use census_domain::model::{CensusRun, GradYear, HistoryWindow, SchoolYear};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture(
) -> Result<(RunManifest, NationalRequest, TeamsSourceRequest), Box<dyn std::error::Error>> {
    let source = request()?;
    let bound = bound_national_request(&source)?;
    let manifest = RunManifest {
        store_identity: "a".repeat(64),
        run: CensusRun::new(bound.season, bound.revision.get()).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: bound.jurisdictions.clone(),
    };
    Ok((manifest, bound, source))
}

#[test]
fn exact_original_request_matches_the_existing_bound_workflow() -> TestResult {
    let (manifest, bound, source) = fixture()?;
    require_scope(&manifest, &bound, &source).map_err(crate::restate_services::tests::sdk_error)?;
    Ok(())
}

#[test]
fn every_changed_acquisition_setting_is_refused_before_effects() -> TestResult {
    let (manifest, bound, original) = fixture()?;
    let baseline = serde_json::to_value((&manifest, &bound, &original))?;
    let mutations: [fn(&mut TeamsSourceRequest) -> TestResult; 10] = [
        |s| {
            s.jurisdiction.history = HistoryWindow::cohort("2026-10-03")?;
            Ok(())
        },
        |s| {
            s.jurisdiction.refresh = !s.jurisdiction.refresh;
            Ok(())
        },
        |s| {
            s.jurisdiction
                .authorized_hosts
                .push("unapproved.example".into());
            Ok(())
        },
        |s| {
            s.jurisdiction.source_parallelism += 1;
            Ok(())
        },
        |s| {
            s.jurisdiction.concurrency += 1;
            Ok(())
        },
        |s| {
            s.jurisdiction.limit_per_state = Some(17);
            Ok(())
        },
        |s| {
            s.jurisdiction.observed_on = Some("2026-10-03".into());
            Ok(())
        },
        |s| {
            s.jurisdiction.season = SchoolYear::new(2027).ok_or("season")?;
            Ok(())
        },
        |s| {
            s.jurisdiction.revision = Revision(2);
            Ok(())
        },
        |s| {
            s.jurisdiction.jurisdiction = UsJurisdiction::Michigan;
            Ok(())
        },
    ];
    for (index, mutate) in mutations.into_iter().enumerate() {
        let mut changed = original.clone();
        mutate(&mut changed)?;
        let changed_before = serde_json::to_value(&changed)?;
        check!(
            require_scope(&manifest, &bound, &changed).is_err(),
            "accepted scope mutation {index}"
        );
        check!(eq; serde_json::to_value(&changed)?, changed_before);
        check!(eq; serde_json::to_value((&manifest, &bound, &original))?, baseline);
    }
    Ok(())
}

#[test]
fn another_workflow_scope_or_run_is_not_an_original_request_authority() -> TestResult {
    let (manifest, bound, original) = fixture()?;
    let mut changed = bound.clone();
    changed.jurisdictions.push(UsJurisdiction::Michigan);
    check!(require_scope(&manifest, &changed, &original).is_err());
    changed.jurisdictions = vec![UsJurisdiction::Michigan];
    check!(require_scope(&manifest, &changed, &original).is_err());
    changed.jurisdictions = vec![UsJurisdiction::Ohio, UsJurisdiction::Ohio];
    check!(require_scope(&manifest, &changed, &original).is_err());
    changed.jurisdictions.clear();
    check!(require_scope(&manifest, &changed, &original).is_err());
    changed = bound.clone();
    changed.revision = Revision(2);
    check!(require_scope(&manifest, &changed, &original).is_err());
    changed = bound;
    changed.season = SchoolYear::new(2027).ok_or("season")?;
    check!(require_scope(&manifest, &changed, &original).is_err());
    Ok(())
}
