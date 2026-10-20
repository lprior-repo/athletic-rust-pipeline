use super::*;
use crate::restate_services::jobs::selected_fixture;
use census_domain::model::{ProfileSelectionV1, QueryProfileV1};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request(selection: Option<ProfileSelectionV1>) -> BindRunRequest {
    BindRunRequest {
        season: 2026,
        revision: 1,
        jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
        selection,
    }
}

#[test]
fn selected_2026_binding_preserves_profile_and_refuses_reinterpretation() -> TestResult {
    let controlled = selected_fixture::install().map_err(|error| error.to_string())?;
    let store = controlled.store();
    let identity = store.store_identity()?;
    let original = controlled.selection().map_err(|error| error.to_string())?;
    let original_profile = store.snapshot().selected_profile(&original)?;
    let mut input = original_profile.input().clone();
    let cohort = GradYear::new(2026).ok_or("cohort")?;
    input.graduation_years = vec![cohort];
    let alternate = QueryProfileV1::new(input)?;
    store.register_profile(&alternate)?;
    let selection = ProfileSelectionV1::new(store.store_identity()?, alternate.id()?);
    let reply = bind(store, request(Some(selection)))?;
    let retained = store.run_manifest()?.ok_or("manifest")?;
    check!(eq; retained.cohort, cohort);
    let expected_run =
        CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, 1).ok_or("run")?;
    check!(eq; retained.run, expected_run);
    check!(eq; retained.jurisdictions, UsJurisdiction::CENSUS_SCOPE.to_vec());
    check!(eq; reply.cohort, 2026);
    check!(eq; retained.store_identity, identity.as_str());
    check!(matches!(
        bind(store, request(Some(original.clone()))),
        Err(JobError::Terminal { .. })
    ));
    let before_replay = store.snapshot().sequence();
    let replay = bind(store, request(None))?;
    check!(eq; replay.cohort, 2026);
    check!(eq; store.snapshot().sequence(), before_replay);
    check!(eq; store.run_manifest()?, Some(retained));
    check!(eq; store.snapshot().selected_profile(&original)?, original_profile);
    Ok(())
}

#[test]
fn missing_selection_requires_and_preserves_an_exact_retained_binding() -> TestResult {
    let controlled = selected_fixture::install().map_err(|error| error.to_string())?;
    let store = controlled.store();
    check!(matches!(
        bind(store, request(None)),
        Err(JobError::Terminal { .. })
    ));
    check!(store.run_manifest()?.is_none());
    let selection = controlled.selection().map_err(|error| error.to_string())?;
    bind(store, request(Some(selection)))?;
    let retained = store.run_manifest()?.ok_or("manifest")?;
    let replay = bind(store, request(None))?;
    check!(eq; replay.cohort, GradYear::CO2027.get());
    check!(eq; replay.store_identity, retained.store_identity);
    let mut foreign = request(None);
    foreign.jurisdictions = vec![UsJurisdiction::Michigan];
    check!(matches!(
        bind(store, foreign),
        Err(JobError::Terminal { .. })
    ));
    check!(eq; store.run_manifest()?, Some(retained));
    Ok(())
}
