use super::*;
use crate::restate_services::jobs::selected_fixture;
use census_domain::model::{ProfileId, ProfileSelectionV1, QueryProfileV1, StoreIdentity};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request(selection: ProfileSelectionV1) -> BindRunRequest {
    BindRunRequest {
        season: 2026,
        revision: 1,
        jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
        selection: Some(selection),
    }
}

fn refuses_without_mutation(store: &Store, selections: &[ProfileSelectionV1]) -> TestResult {
    for selection in selections {
        let manifest = store.run_manifest()?;
        let sequence = store.snapshot().sequence();
        check!(matches!(
            bind(store, request(selection.clone())),
            Err(JobError::Terminal { .. })
        ));
        check!(eq; store.run_manifest()?, manifest);
        check!(eq; store.snapshot().sequence(), sequence);
    }
    Ok(())
}

#[test]
fn foreign_unknown_and_multi_cohort_requests_cannot_mutate_fresh_or_bound_runs() -> TestResult {
    let controlled = selected_fixture::install().map_err(|error| error.to_string())?;
    let store = controlled.store();
    let identity = store.store_identity()?;
    let foreign = StoreIdentity::parse(&"a".repeat(64))?;
    check!(ne; foreign, identity);
    let mut input = controlled.profile().input().clone();
    input.graduation_years = vec![GradYear::new(2026).ok_or("cohort")?, GradYear::CO2027];
    let multi = QueryProfileV1::new(input.clone())?;
    store.register_profile(&multi)?;
    input.graduation_years = vec![GradYear::new(2026).ok_or("cohort")?];
    let single = QueryProfileV1::new(input)?;
    store.register_profile(&single)?;
    let invalid = [
        ProfileSelectionV1::new(foreign, controlled.profile().id()?),
        ProfileSelectionV1::new(identity.clone(), ProfileId::parse(&"b".repeat(64))?),
        ProfileSelectionV1::new(identity.clone(), multi.id()?),
    ];
    refuses_without_mutation(store, &invalid)?;
    check!(store.run_manifest()?.is_none());
    let reply = bind(
        store,
        request(ProfileSelectionV1::new(identity, single.id()?)),
    )?;
    check!(eq; reply.cohort, 2026);
    let retained = store.run_manifest()?.ok_or("bound run")?;
    check!(eq; retained.cohort.get(), 2026);
    refuses_without_mutation(store, &invalid)?;
    check!(eq; store.run_manifest()?, Some(retained));
    Ok(())
}
